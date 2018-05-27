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

// <https://encode.ru/threads/
//  1698-Fast-CRC-table-construction-and-rolling-CRC-hash-calculation>

// Original attribution
// crc.c -- Fast CRC table construction and rolling CRC hash calculation
// 2009-11-23 : Igor Pavlov : Public domain
// 2013-03-27 : Bulat.Ziganshin@gmail.com : Public domain

#[cfg(test)]
extern crate crc;

/// Standard CRC-32 IEEE *et al* polynomial.
pub const POLY_CRC: u32 =  0xEDB88320;

// This constant is "0xFFFFFFFF for zip/rar/7-zip
// 'quasi-CRC'" according to the original source; I'm not
// sure what's "quasi" about it, though.

/// Constant used as an "initial value" for the hash, and
/// XORed with the hash before returning it.
///
/// A choice of all-ones is the IEEE *et al* standard. This
/// is nice for testing and for compatibility with hashes
/// produced by other programs.
///
/// A choice of 0 will give a much faster rolling hash table
/// build (no impact on rolling hash performance) at the
/// expense of testing and compatibility.
pub const INIT_CRC: u32 = !0;

/// A CRC table is just an array of 256 CRC values; one per
/// possible byte value.
pub(crate) type CRCTable = [u32; 256];

/// Given the current CRC, return the CRC including the
/// next character.
#[inline(always)]
pub(crate) fn update_crc(crc: u32, crc_table: &CRCTable, c: u8) -> u32 {
    crc_table[((crc ^ (c as u32)) & 0xff) as usize] ^ (crc >> 8)
}

/// Apply INIT_CRC to the final CRC. This can also be
/// used to remove this value to continue a closed hash.
#[inline(always)]
pub(crate) fn finish_crc(crc: u32) -> u32 {
    crc ^ INIT_CRC
}

/// Calculate a standard (non-rolling) CRC of the given
/// buffer.
pub fn calc_crc(buf: &[u8], crc_table: &CRCTable) -> u32 {
  let mut crc = INIT_CRC;
  for c in buf {
      crc = update_crc(crc, crc_table, *c);
  }
  finish_crc(crc)
}

// This construction allows computing the standard
// (non-running) CRC table with a reasonable amount of
// work. If table computation is a concern, and
// INIT_CRC is fixed / know, this table could be
// built once and compiled into the code.
//
// I haven't analyzed this algorithm and don't understand
// it, but it seems to work.

/// Fast CRC table construction algorithm.
///
/// The "seed" here is only used by the fast running CRC
/// table computation below: it is normal to pass the hash
/// polynomial `CRC32_IEEE`.
pub(crate) fn make_crc_table(crc_table: &mut CRCTable, seed: u32) {
    let mut r = seed;
    crc_table[0] = 0;
    crc_table[128] = seed;

    let mut i = 64;
    while i > 0 {
        r = (r >> 1) ^ (POLY_CRC & !(u32::wrapping_sub(r & 1, 1)));
        crc_table[i] = r;
        i >>= 1;
    }

    i = 2;
    while i < 256 {
        for j in 1..i {
            crc_table[i+j] = crc_table[i] ^ crc_table[j];
        }
        i <<= 1;
    }
}

#[test]
fn test_fast_crc_table() {
    // Fast CRC table construction
    let mut fast_crc_table = [0; 256];
    make_crc_table(&mut fast_crc_table, POLY_CRC);

    // Classic CRC table construction algorithm
    let mut crc_table = [0; 256];
    for i in 0..256 {
        let mut r = i;
        for _ in 0..8 {
            r = (r >> 1) ^ (POLY_CRC & !(u32::wrapping_sub(r & 1, 1)));
        }
        crc_table[i as usize] = r;
    }

    assert_eq!(&fast_crc_table as &[u32], &crc_table as &[u32]);
}

// This next bit deserves a careful explanation.
//
// For any messages X and Y of the same length,
// the linearity of CRC gives us that
//
//        CRC(X ^ Y) == CRC(X) ^ CRC(Y)
//
// Here, the length of X and Y is one more than the window
// size w. We will be setting up to remove x, the first byte
// of X, and add a new byte at the end.  To do this, we will
// compute this table.
//
// For each possible byte x, we construct the CRC of a
// message consisting of just x followed by w zero bytes. We
// also construct the CRC of a message Y that consists of w
// zero bytes.  The XOR of these two CRCs effectively
// removes the effect of the initial zero byte, and sets up
// for XOR-ing the CRC of a trailing byte y into the current
// rolling CRC so that that we have effectively rolled the
// CRC one byte forward.
//
// In addition to starting the CRC at all-ones, the standard
// IEEE CRC algorithm complements the CRC at the end. This
// table leaves the CRC "open", such that the complement
// hasn't been done, which is useful for continuing a
// rolling CRC. To "close" it, call `finish_crc()` above
// on the current CRC.

/// Make a rolling CRC table for the given window size.
/// This requires first computing the standard CRC table.
fn make_rolling_crc_table_slow(winsize: usize,
                               crc_table: &CRCTable,
                               rolling_crc_table: &mut CRCTable)
{
    for c in 0..=255 {
        let mut x = INIT_CRC;
        let mut y = INIT_CRC;
        x = update_crc(x, crc_table, c);
        y = update_crc(y, crc_table, 0);
        for _ in 0..winsize-1 {
            x = update_crc(x, &crc_table, 0);
            y = update_crc(y, &crc_table, 0);
        }
        x = update_crc(x, crc_table, 0);
        rolling_crc_table[c as usize] = x ^ y;
    }
}

/// Fast rolling CRC table construction algorithm; use only
/// when INIT_CRC == 0.
fn make_rolling_crc_table_fast(winsize: usize,
                               crc_table: &CRCTable,
                               rolling_crc_table: &mut CRCTable)
{
    assert!(INIT_CRC == 0);

    let mut crc = INIT_CRC;
    crc = update_crc(crc, &crc_table, 128);
    for _ in 0..winsize {
        crc = update_crc(crc, &crc_table, 0);
    }
    crc = finish_crc(crc);

    make_crc_table(rolling_crc_table, crc);
}

/// Make a rolling CRC table for the given window size.
/// This requires first computing the standard CRC table.
pub(crate) fn make_rolling_crc_table(winsize: usize,
                                     crc_table: &CRCTable,
                                     rolling_crc_table: &mut CRCTable)
{
    if INIT_CRC == 0 {
        make_rolling_crc_table_fast(winsize, crc_table, rolling_crc_table);
    } else {
        make_rolling_crc_table_slow(winsize, crc_table, rolling_crc_table);
    }
}

#[test]
fn test_rolling_crc_table() {
    // Make the base CRC table.
    let mut crc_table = [0; 256];
    make_crc_table(&mut crc_table, POLY_CRC);

    // Try rolling a variety of window sizes.
    for winsize in 2..16 {

        // Rolling CRC table construction.
        let mut rolling_crc_table = [0; 256];
        make_rolling_crc_table(winsize,
                               &crc_table,
                               &mut rolling_crc_table);

        // Test fast rolling CRC table construction if in
        // use.
        if INIT_CRC == 0 {
            let mut slow_rolling_crc_table = [0; 256];
            make_rolling_crc_table_slow(winsize,
                                        &crc_table,
                                        &mut slow_rolling_crc_table);
            assert_eq!(&rolling_crc_table as &[u32],
                       &slow_rolling_crc_table as &[u32]);
        }

        let test_size = 2 * winsize;
        // Make a buffer of "random" values.
        let buffer: Vec<u8> = (0..winsize+test_size)
            .map(|i| ((11 + i*31 + i/17) & 0xff) as u8)
            .collect();

        // Calculate the CRC of the tail of the buffer using
        // the rolling hash and check for agreement.

        // Get the initial hash.
        let mut crc2 =
            calc_crc(&buffer[0..winsize], &crc_table);
        // Open the rolling hash.
        crc2 = finish_crc(crc2);
        // Run rolling and regular hash over remaining
        // windows of buffer.
        for i in winsize..winsize+test_size {
            // Get a slice containing the current window.
            let window = &buffer[i-winsize+1..=i];
            // Directly calculate the target hash.
            let crc1 = calc_crc(window, &crc_table);
            // If in the standard case, make sure the target
            // hash agrees with third-party calculation.
            if INIT_CRC == !0 {
                let crcx = crc::crc32::checksum_ieee(window);
                assert_eq!(crc1, crcx);
            }
            // Roll the hash.
            crc2 = update_crc(crc2, &crc_table, buffer[i])
                ^ rolling_crc_table[buffer[i - winsize] as usize];
            // Ensure that the closed rolling hash agrees
            // with the target hash.
            if crc1 != finish_crc(crc2) {
                panic!("{:08x} != {:08x} ({} {})",
                       crc1, crc2, winsize, i);
            }
        }
    }
}
