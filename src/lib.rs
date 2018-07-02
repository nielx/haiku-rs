//
// Copyright 2015, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![crate_name = "haiku"]

extern crate haiku_sys;
extern crate libc;

pub mod kernel;
pub mod storage;
