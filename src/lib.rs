//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![crate_name = "haiku"]

//! This crate contains high-level bindings for Haiku
//!
//! The goal is to make various low-level Haiku API's available for use in
//! Rust, with both the safety and the usability of the Rust standard
//! libraries. 
//!
//! This crate is very much work in progress.


extern crate haiku_sys;
extern crate libc;

pub mod app;
pub mod kernel;
pub mod storage;

