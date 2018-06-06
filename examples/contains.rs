// Copyright © 2018 Bart Massey
// [This program is licensed under the "MIT License"]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! Check for containment of a string in files.
//! This is essentially Aho-Corasick with CRC hashing.
//! It is not expected to be especially fast.

extern crate filebuffer;
extern crate rolling_crc;

use rolling_crc::*;

use std::io::{self, Read};

fn main() -> Result<(), io::Error> {
    // Set up.
    let mut args = std::env::args().peekable();
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
    let rcrc = RollingCRC::new(&context);
    let target_crc = context.crc(target.as_bytes());

    // Read from stdin.
    if args.peek() == None {
        let stdin = io::stdin();
        let bytes = io::BufReader::new(stdin).bytes();
        for result in rcrc.iter_result(bytes) {
            let (index, crc) = result?;
            if crc == target_crc {
                println!("{}", index);
            }
        }
        return Ok(());
    }

    // Read from files.
    for filename in args {
        let f = filebuffer::FileBuffer::open(&filename)?;
        f.prefetch(0, f.len());
        let rcrc = rcrc.clone();
        for result in rcrc.iter_result(f.bytes()) {
            let (index, crc) = result?;
            if crc == target_crc {
                println!("{}: {}", filename, index);
            }
        }
    }
    Ok(())
}
