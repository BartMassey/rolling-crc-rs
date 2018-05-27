// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! Check for containment of a string in files.
//! This is essentially Aho-Corasick with CRC hashing.
//! It is not expected to be especially fast.

extern crate rolling_crc;
use rolling_crc::*;

use std::io;
use std::io::Read;

fn main() -> Result<(), io::Error> {
    let mut args = std::env::args();
    let _ = args.next();
    let target = match args.next() {
        None =>
            return Err(io::Error::new(
                io::ErrorKind::Other, "no target specified")),
        Some(ref target) if target.len() == 0 =>
            return Err(io::Error::new(
                io::ErrorKind::Other, "empty target")),
        Some(target) => target,
    };
    let context = RollingCRCContext::new(target.len());
    let target_crc = context.crc(target.as_bytes());
    for filename in args {
        let f = std::fs::File::open(&filename)?;
        let bytes = io::BufReader::new(f).bytes();
        let iter = RollingCRC::new(&context).iter_result(bytes);
        for result in iter {
            let (index, crc) = result?;
            if crc == target_crc {
                println!("{}: {}", filename, index);
            }
        }
    }
    Ok(())
}
