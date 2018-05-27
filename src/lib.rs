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
