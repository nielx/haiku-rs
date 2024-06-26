//
// Copyright 2018, 2024, Niels Sascha Reedijk <niels.reedijk@gmail.com>
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

#[macro_use]
extern crate lazy_static;
extern crate libc;

pub mod app;
pub mod kernel;
pub mod storage;
pub mod support;

#[macro_export]
macro_rules! haiku_constant {
	($a:tt, $b:tt, $c:tt, $d:tt) => {
		(($a as u32) << 24) + (($b as u32) << 16) + (($c as u32) << 8) + ($d as u32)
	};
}
