#![allow(unused)]

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
// 2013-03-27 : Bulat.Ziganshin@gmail.com : Public domain */
 
#[cfg(test)]
extern crate crc;

/// Standard CRC-32 polynomial
const CRC_POLY: u32 =  0xEDB88320;

/// 0xFFFFFFFF for zip/rar/7-zip "quasi-CRC" (?).
///
/// This can be set to 0 and the code changed below for a
/// slightly faster implementation, but this implementation
/// yields normal CRC-32 within the window, which is nice
/// for compatibility and testing.
const CRC_INIT_VAL: u32 = !0;
 
type CRCTable = [u32; 256];

/// Given the current CRC, return the CRC including the
/// next character.
#[inline(always)]
pub fn update_crc(crc: u32, crc_table: &CRCTable, c: u8) -> u32 {
    crc_table[((crc ^ (c as u32)) & 0xff) as usize] ^ (crc >> 8)
}

/// Apply CRC_INIT_VAL to the final CRC.
#[inline(always)]
pub fn finish_crc(crc: u32) -> u32 {
    crc ^ CRC_INIT_VAL
}

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
        for _ in 0..winsize { 
            x = update_crc(x, &crc_table, 0);
            y = update_crc(y, &crc_table, 0);
        } 
        rolling_crc_table[c as usize] = finish_crc(x) ^ finish_crc(y);
    } 
}


/// Fast CRC table construction algorithm.
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
fn _make_rolling_crc_table_fast(winsize: usize,
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
        for j in 0..8 {
            r = (r >> 1) ^ (CRC_POLY & !(u32::wrapping_sub(r & 1, 1)));
        }
        crc_table[i as usize] = r;
    }
    
    assert_eq!(&fast_crc_table as &[u32], &crc_table as &[u32]);
}
 
#[ignore]
/// This code got in here somewhere. I don't know how.
fn _make_rolling_crc_table_mystery(winsize: usize,
                                   crc_table: &CRCTable,
                                   rolling_crc_table: &mut CRCTable)
{
    for i in 0..=255 {
        let mut crc = CRC_INIT_VAL;
        crc = update_crc(crc, &crc_table, i);
        for j in 0..winsize {
            crc = update_crc(crc, &crc_table, 0);
        }
        rolling_crc_table[i as usize] = finish_crc(crc);
    }
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
            _make_rolling_crc_table_fast(winsize,
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
        let mut crc2 =
            calc_crc(&buffer[0..winsize], &crc_table);
        for i in 0..test_size {
            let window = &buffer[i+1..=i+winsize];
            let crc1 = calc_crc(window, &crc_table);
            if CRC_INIT_VAL == !0 {
                let crcx = crc::crc32::checksum_ieee(window);
                assert_eq!(crc1, crcx);
            }
            crc2 = update_crc(crc2, &crc_table, buffer[i+winsize])
                ^ rolling_crc_table[buffer[i] as usize];
            if crc1 != crc2 {
                panic!("{:08x} != {:08x} ({} {})",
                       crc1, crc2, winsize, i);
            }
        }
    }
}
