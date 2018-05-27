# rolling-crc-rs
Copyright (c) 2018 Bart Massey

Code for computing rolling CRC hashes. A rolling hash
consists of a stream of hashes of successive fixed-size
windows of a data stream, but can be computed in fast
constant time per hash independent of the window size.

This code is originally derived from a C version by Bulat
Zuganshin *et al.* See
<http://github.com/BartMassey/rolling-crc> for that version.

This work is made available under the "MIT License". Please
see the file `LICENSE` in this distribution for license
terms.
