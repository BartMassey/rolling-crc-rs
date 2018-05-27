// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! Demo of basic functionality.

extern crate rolling_crc;
use rolling_crc::*;

fn main() {
    let context = RollingCRCContext::new(3);
    let mut roll_crc = RollingCRC::new(&context);
    let bytes = "hello world".as_bytes();
    for i in 0..bytes.len() {
        let crc = roll_crc.push(bytes[i]);
        if i < 2 {
            println!("{} {:?}", i, crc);
        } else {
            let true_crc = context.crc(&bytes[i-2..=i]);
            let crc = crc.unwrap();
            println!("{} {:08x?} {:08x?}", i, crc, true_crc);
        }
    }
}
