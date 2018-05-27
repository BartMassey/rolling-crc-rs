// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! Implementation of rolling CRC-32 using the "standard"
//! cyclic polynomial (ISO 3309 etc) based on Igor Pavlov
//! and Bulat Ziganshin's public domain code. See the source
//! for full attribution.

// <https://encode.ru/threads/
//  1698-Fast-CRC-table-construction-and-rolling-CRC-hash-calculation>

// Original attribution
// crc.c -- Fast CRC table construction and rolling CRC hash calculation
// 2009-11-23 : Igor Pavlov : Public domain
// 2013-03-27 : Bulat.Ziganshin@gmail.com : Public domain
 
#[cfg(test)]
extern crate crc;

/// Standard CRC-32 polynomial
const CRC_POLY: u32 =  0xEDB88320;

/// "0xFFFFFFFF for zip/rar/7-zip "quasi-CRC" (?)."
///
/// This can be set to 0 and the code changed below for a
/// much faster rolling hash table build (no impact on
/// rolling hash performance).
///
/// The current choice yields normal CRC-32 within the
/// window, which is nice for compatibility and testing.
const CRC_INIT_VAL: u32 = !0;
 
type CRCTable = [u32; 256];

/// Given the current CRC, return the CRC including the
/// next character.
#[inline(always)]
pub fn update_crc(crc: u32, crc_table: &CRCTable, c: u8) -> u32 {
    crc_table[((crc ^ (c as u32)) & 0xff) as usize] ^ (crc >> 8)
}

/// Apply CRC_INIT_VAL to the final CRC. This can also be
/// used to remove this value to continue a closed hash.
#[inline(always)]
pub fn finish_crc(crc: u32) -> u32 {
    crc ^ CRC_INIT_VAL
}

// This next bit deserves a careful explanation.
//
// For any messages X and Y of the same length,
// the linearity of CRC gives us that
//
//        CRC(X^Y) == CRC(X)^CRC(Y)
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

/// Make a rolling CRC table for the given window size using
/// the standard CRC table.
pub fn make_rolling_crc_table(winsize: usize,
                          crc_table: &CRCTable,
                          rolling_crc_table: &mut CRCTable)
{ 
    for c in 0..=255 {
        let mut x = CRC_INIT_VAL;
        let mut y = CRC_INIT_VAL; 
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


// This construction allows computing the standard
// (non-running) CRC table with a reasonable amount of
// work. If table computation is a concern, and
// CRC_INIT_VALUE is fixed / know, this table could be built
// once and compiled into the code.
//
// I haven't analyzed this algorithm and don't understand
// it, but it seems to work.

/// Fast CRC table construction algorithm.
///
/// The "seed" here is only used by the fast running CRC
/// table computation below: it is normal to pass the hash
/// polynomial `CRC_POLY`.
pub fn make_crc_table_fast(crc_table: &mut CRCTable, seed: u32) {
    let mut r = seed;
    crc_table[0] = 0;
    crc_table[128] = seed;

    let mut i = 64;
    while i > 0 {
        r = (r >> 1) ^ (CRC_POLY & !(u32::wrapping_sub(r & 1, 1)));
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
 
/// Fast rolling CRC table construction algorithm; use only
/// when CRC_INIT_VAL == 0.
#[cfg(test)]
fn make_rolling_crc_table_fast(winsize: usize,
                                rolling_crc_table: &mut CRCTable)
{
    assert!(CRC_INIT_VAL == 0);

    let mut crc_table = [0;256];
    make_crc_table_fast(&mut crc_table, CRC_POLY);

    let mut crc = CRC_INIT_VAL;
    crc = update_crc(crc, &crc_table, 128);
    for _ in 0..winsize {
        crc = update_crc(crc, &crc_table, 0);
    }
    crc = finish_crc(crc);

    make_crc_table_fast(rolling_crc_table, crc);
}

/// Calculate a standard (non-rolling) CRC of the given
/// buffer.
pub fn calc_crc(buf: &[u8], crc_table: &CRCTable) -> u32 {
  let mut crc = CRC_INIT_VAL;
  for c in buf {
      crc = update_crc(crc, crc_table, *c);
  }
  finish_crc(crc)
}
 
#[test]
fn test_fast_crc_table() {
 
    // Fast CRC table construction
    let mut fast_crc_table = [0; 256];
    make_crc_table_fast(&mut fast_crc_table, CRC_POLY);
 
    // Classic CRC table construction algorithm
    let mut crc_table = [0; 256];
    for i in 0..256 {
        let mut r = i;
        for _ in 0..8 {
            r = (r >> 1) ^ (CRC_POLY & !(u32::wrapping_sub(r & 1, 1)));
        }
        crc_table[i as usize] = r;
    }
    
    assert_eq!(&fast_crc_table as &[u32], &crc_table as &[u32]);
}
 
#[test]
fn test_rolling_crc_table() {
    let mut crc_table = [0; 256];
    make_crc_table_fast(&mut crc_table, CRC_POLY);

 
    // Try for a variety of window sizes.
    for winsize in 2..16 {

        // Rolling CRC table construction.
        let mut rolling_crc_table = [0; 256];
        make_rolling_crc_table(winsize,
                               &crc_table,
                               &mut rolling_crc_table);

        // Optionally test fast rolling CRC table construction.
        if CRC_INIT_VAL == 0 {
            let mut fast_rolling_crc_table = [0; 256];
            make_rolling_crc_table_fast(winsize,
                                         &mut fast_rolling_crc_table);
            assert_eq!(&rolling_crc_table as &[u32],
                       &fast_rolling_crc_table as &[u32]);
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
            let window = &buffer[i-winsize..i];
            // Directly calculate the target hash.
            let crc1 = calc_crc(window, &crc_table);
            // If in the standard case, make sure the target
            // hash agrees with third-party calculation.
            if CRC_INIT_VAL == !0 {
                let crcx = crc::crc32::checksum_ieee(window);
                assert_eq!(crc1, crcx);
            }
            // Close the rolling hash.
            crc2 = finish_crc(crc2);
            // Ensure that the closed rolling hash agrees
            // with the target hash.
            if crc1 != crc2 {
                panic!("{:08x} != {:08x} ({} {})",
                       crc1, crc2, winsize, i);
            }
            // Reopen the rolling hash.
            crc2 = finish_crc(crc2);
            // Roll the hash.
            crc2 = update_crc(crc2, &crc_table, buffer[i])
                ^ rolling_crc_table[buffer[i - winsize] as usize];
        }
    }
}
