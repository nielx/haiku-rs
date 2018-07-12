//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

//! Module for flattening and unflattening data
//!
//! To be documented


/// An interface for types that are flattenable
pub trait Flattenable {
	/// The type code is a unique identifier that identifies the flattened data
	fn type_code(&self) -> u32;
	/// Return the size of the flattened type
	fn flattened_size(&self) -> isize;
	/// Check if flattened objects of this type are always a fixed size
	fn is_fixed_size(&self) -> bool;
	/// Copy a flattened version of this object into the stream
	fn flatten(&self, 
}

