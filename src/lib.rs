// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! Implementation of rolling CRC-32 using the "standard"
//! cyclic polynomial (ISO 3309 etc).
//!
//! A rolling hash consists of a stream of hashes of
//! successive fixed-size windows of a data stream, but can
//! be computed in fast constant time per hash independent
//! of the window size.
//!
//! This work is based on Igor Pavlov and Bulat Ziganshin's
//! public domain code. See the source for full attribution;
//! it is also available as
//! <http://github.com/BartMassey/rolling-crc>.

mod rollcrc;
pub use rollcrc::*;

#[macro_use]
extern crate lazy_static;

use std::fmt;

// Build the CRC table just once at first use.  It is not
// clear to me where the performance penalty for referencing
// this lives.
lazy_static! {
    static ref CRC_TABLE: CRCTable = {
        let mut crc_table = [0;256];
        make_crc_table(&mut crc_table, POLY_CRC);
        crc_table
    };
}

/// Data needed for rolling CRC calculation.
#[derive(Clone)]
pub struct RollingCRCContext<'a> {
    /// Size of calculation window.
    window_size: usize,
    /// CRC table.
    crc_table: &'a CRCTable,
    /// Rolling CRC table for this window size.
    rolling_crc_table: CRCTable,
}

impl<'a> fmt::Debug for RollingCRCContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RollingCRCContext {{ \
                   window_size: {}",
               self.window_size)?;
        write!(f, ", crc_table: ")?;
        self.crc_table[..].fmt(f)?;
        write!(f, ", rolling_crc_table: ")?;
        self.rolling_crc_table[..].fmt(f)?;
        write!(f, " }}")
    }
}

impl<'a> RollingCRCContext<'a> {

    /// Make a new rolling CRC context for this window size.
    /// The first call will incur the overhead of CRC table
    /// calculation. Subsequent calls will incur the
    /// overhead of rolling CRC table calculation.
    pub fn new(window_size: usize) -> Self {
        let crc_table = &CRC_TABLE;
        let mut rolling_crc_table = [0; 256];
        if window_size >= 1 {
            make_rolling_crc_table(
                window_size,
                &crc_table,
                &mut rolling_crc_table,
                );
        }
        Self { window_size, crc_table, rolling_crc_table }
    }

    /// Compute the CRC of the given bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rolling_crc::*;
    /// let context = RollingCRCContext::new(0);
    /// let bytes = "hello world".as_bytes();
    /// assert_eq!(context.crc(bytes), 0x0d4a1185);
    /// ```
    pub fn crc(&self, bytes: &[u8]) -> u32 {
        calc_crc(bytes, &self.crc_table)
    }

}

/// An in-progress rolling CRC.
#[derive(Debug, Clone)]
pub struct RollingCRC<'a> {
    /// Needed context information.
    context: &'a RollingCRCContext<'a>,
    /// Number of bytes processed so far.
    count: usize,
    /// Bytes in window.
    bytes: Vec<u8>,
    /// Index of next byte in window to be replaced. We
    /// implement our own circular queue, to avoid the
    /// overhead of calls to the standard one.
    index: usize,
    /// Last "open" rolling CRC, to continue rolling.
    last_crc: Option<u32>,
}

impl<'a> RollingCRC<'a> {

    /// Start a new rolling CRC in the given context. If the
    /// window size of `context` is 0, this structure
    /// will never return a rolling CRC.
    pub fn new(context: &'a RollingCRCContext<'a>) -> Self {
        Self {
            context,
            count: 0,
            bytes: Vec::new(),
            index: 0,
            last_crc: None,
        }
    }

    /// Roll a byte through this rolling CRC. This is likely
    /// to be pretty expensive per-byte, but it can be
    /// convenient.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rolling_crc::*;
    /// let context = RollingCRCContext::new(2);
    /// let mut roll_crc = RollingCRC::new(&context);
    /// let bytes = "hello world".as_bytes();
    /// for i in 0..bytes.len() {
    ///     let crc = roll_crc.push(bytes[i]);
    ///     if i == 0 {
    ///         assert_eq!(crc, None);
    ///     } else {
    ///         assert_eq!(crc, Some(context.crc(&bytes[i-1..=i])));
    ///     }
    /// }
    /// ```
    #[inline(always)]
    pub fn push(&mut self, byte: u8) -> Option<u32> {
        self.count += 1;
        if self.context.window_size == 0 {
            return None;
        }
        if self.count < self.context.window_size {
            self.bytes.push(byte);
            return None;
        }
        if self.count == self.context.window_size {
            self.bytes.push(byte);
            let crc = self.context.crc(&self.bytes);
            self.last_crc = Some(finish_crc(crc));
            return Some(crc);
        }
        assert!(self.context.window_size == self.bytes.len());
        let roll_out = self.bytes[self.index] as usize;
        let last_crc = self.last_crc.expect("internal error: lost CRC");
        let table = self.context.crc_table;
        let rolling_table = self.context.rolling_crc_table;
        let crc = update_crc(last_crc, &table, byte) ^ rolling_table[roll_out];
        self.bytes[self.index] = byte;
        self.index += 1;
        if self.index >= self.context.window_size {
            self.index = 0;
        }
        self.last_crc=Some(crc);
        Some(finish_crc(crc))
    }

    /// An iterator over the bytes from the given byte
    /// iterator, returning successive rolling CRCs
    /// resulting from operating on the given state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rolling_crc::*;
    /// // Set up the byte source.
    /// let s = "hello world";
    /// let ns = s.len();
    /// // Set up the collection context.
    /// let context = RollingCRCContext::new(5);
    /// let rolling_crc = RollingCRC::new(&context);
    /// // Collect the rolling CRCs.
    /// let mut crcs: Vec<(usize, u32)> =
    ///     rolling_crc.iter(s.bytes()).collect();
    /// // Check the result.
    /// assert_eq!(ns - 4, crcs.len());
    /// let bytes = s.as_bytes();
    /// let crc = context.crc(&bytes[ns-5..ns]);
    /// assert_eq!(Some((ns - 5, crc)), crcs.pop());
    /// ```
    pub fn iter<T>(self, bytes: T) -> RollingCRCMap<'a, T>
        where T: Iterator<Item=u8>
    {
        RollingCRCMap{ rolling_crc: self, bytes }
    }

    /// A version of `iter` that preserves errors in its
    /// input byte results.
    pub fn iter_result<T, E>(self, bytes: T) ->
        RollingCRCMapResult<'a, T, E>
        where T: Iterator<Item=Result<u8, E>>
    {
        RollingCRCMapResult{ rolling_crc: self, bytes }
    }
}

/// An iterator that maps the stream of input bytes from the
/// given byte iterator to a stream of positions relative to
/// the start of a given `RollingCRC` and their
/// corresponding rolling CRCs. This iterator can be created
/// using `RollingCRC::iter()`.
#[derive(Debug, Clone)]
pub struct RollingCRCMap<'a, T>
    where T: Iterator<Item=u8>
{
    rolling_crc: RollingCRC<'a>,
    bytes: T,
}

impl<'a, T> Iterator for RollingCRCMap<'a, T>
    where T: Iterator<Item=u8>
{
    type Item = (usize, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let window_size = self.rolling_crc.context.window_size;
        loop {
            let byte = self.bytes.next()?;
            let crc = self.rolling_crc.push(byte);
            if let Some(crc) = crc {
                let index = self.rolling_crc.count - window_size;
                return Some((index, crc));
            }
        }
    }
}

#[test]
fn test_iterator_index() {
    // Set up the byte source.
    let s = b"llllollllollll";
    // Set up the collection context.
    let context = RollingCRCContext::new(4);
    let target = context.crc(b"llll");
    let rolling_crc = RollingCRC::new(&context);
    // Collect the rolling CRCs.
    for (index, crc) in rolling_crc.iter(s.into_iter().cloned()) {
        if index == 0 || index == 5 || index == 10 {
            assert_eq!(crc, target);
        } else {
            assert!(crc != target);
        }
    }
}

/// An iterator that maps the stream of input byte results
/// from the given byte iterator to a stream of position
/// results relative to the start of a given `RollingCRC`
/// and their corresponding rolling CRCs. This iterator can
/// be created using `RollingCRC::iter_result()`.
pub struct RollingCRCMapResult<'a, T, E>
    where T: Iterator<Item=Result<u8, E>>
{
    rolling_crc: RollingCRC<'a>,
    bytes: T,
}

impl<'a, T, E> Iterator for RollingCRCMapResult<'a, T, E>
    where T: Iterator<Item=Result<u8, E>>
{
    type Item = Result<(usize, u32), E>;

    fn next(&mut self) -> Option<Self::Item> {
        let window_size = self.rolling_crc.context.window_size;
        loop {
            let byte = self.bytes.next()?;
            let byte = match byte {
                Ok(byte) => byte,
                Err(e) => return Some(Err(e)),
            };
            let crc = self.rolling_crc.push(byte);
            if let Some(crc) = crc {
                let index = self.rolling_crc.count - window_size;
                return Some(Ok((index, crc)));
            }
        }
    }
}
