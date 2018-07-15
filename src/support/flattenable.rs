//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

//! Module for flattening and unflattening data
//!
//! Flattening is a Haiku concept where all types of data can be stored as and
//! read from a byte stream. It is used in several areas, such as Messages and
//! file attributes. This module implements the concept for Rust, which makes
//! it possible to work with flattened data in Rust. If you want to use the
//! flattening API for your own data, you should implement the Flattenable
//! trait.


/// An interface for types that are flattenable
pub trait Flattenable<T> {
	/// The type code is a unique identifier that identifies the flattened data
	fn type_code(&self) -> u32;
	/// Return the size of the flattened type
	fn flattened_size(&self) -> isize;
	/// Check if flattened objects of this type are always a fixed size
	fn is_fixed_size(&self) -> bool;
	/// Return a flattened version of this object
	fn flatten(&self) -> Vec<u8>;
	/// Unflatten an object from a stream
	fn unflatten(&[u8]) -> Option<T>;
}
