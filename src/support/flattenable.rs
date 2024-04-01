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

use haiku_sys::*;
use std::ffi::{CStr, CString};
use std::mem;

use support::{ErrorKind, HaikuError, Result};

/// An interface for types that are flattenable
pub trait Flattenable<T> {
	/// The type code is a unique identifier that identifies the flattened data
	fn type_code() -> u32;
	/// Check if flattened objects of this type are always a fixed size
	fn is_fixed_size() -> bool;
	/// Return the size of the flattened type
	fn flattened_size(&self) -> usize;
	/// Return a flattened version of this object
	fn flatten(&self) -> Vec<u8>;
	/// Unflatten an object from a stream
	fn unflatten(&[u8]) -> Result<T>;

	// TODO: The Haiku API also implements AllowsTypeCode() for each supported
	// type to for example support unflattening a mime type also as a string
	// type. For now this is not implemented here, as these inferences can be
	// made in the code that uses the API to unflatten.
}

impl Flattenable<bool> for bool {
	fn type_code() -> u32 {
		B_BOOL_TYPE
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flattened_size(&self) -> usize {
		1
	}

	fn flatten(&self) -> Vec<u8> {
		if *self {
			vec![1 as u8]
		} else {
			vec![0 as u8]
		}
	}

	fn unflatten(buffer: &[u8]) -> Result<bool> {
		if buffer.len() != 1 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else if buffer[0] == 0 {
			Ok(false)
		} else {
			Ok(true)
		}
	}
}

impl Flattenable<i8> for i8 {
	fn type_code() -> u32 {
		B_INT8_TYPE
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flattened_size(&self) -> usize {
		1
	}

	fn flatten(&self) -> Vec<u8> {
		vec![*self as u8]
	}

	fn unflatten(buffer: &[u8]) -> Result<i8> {
		if buffer.len() != 1 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer[0] as i8)
		}
	}
}

impl Flattenable<i16> for i16 {
	fn type_code() -> u32 {
		B_INT16_TYPE
	}

	fn flattened_size(&self) -> usize {
		2
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<i16, [u8; 2]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<i16> {
		if buffer.len() != 2 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as i16))
		}
	}
}

impl Flattenable<i32> for i32 {
	fn type_code() -> u32 {
		B_INT32_TYPE
	}

	fn flattened_size(&self) -> usize {
		4
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<i32, [u8; 4]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<i32> {
		if buffer.len() != 4 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as i32))
		}
	}
}

impl Flattenable<i64> for i64 {
	fn type_code() -> u32 {
		B_INT64_TYPE
	}

	fn flattened_size(&self) -> usize {
		8
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<i64, [u8; 8]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<i64> {
		if buffer.len() != 8 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as i64))
		}
	}
}

impl Flattenable<u8> for u8 {
	fn type_code() -> u32 {
		B_UINT8_TYPE
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flattened_size(&self) -> usize {
		1
	}

	fn flatten(&self) -> Vec<u8> {
		vec![*self]
	}

	fn unflatten(buffer: &[u8]) -> Result<u8> {
		if buffer.len() != 1 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer[0])
		}
	}
}

impl Flattenable<u16> for u16 {
	fn type_code() -> u32 {
		B_UINT16_TYPE
	}

	fn flattened_size(&self) -> usize {
		2
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<u16, [u8; 2]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<u16> {
		if buffer.len() != 2 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u16))
		}
	}
}

impl Flattenable<u32> for u32 {
	fn type_code() -> u32 {
		B_UINT32_TYPE
	}

	fn flattened_size(&self) -> usize {
		4
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<u32, [u8; 4]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<u32> {
		if buffer.len() != 4 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u32))
		}
	}
}

impl Flattenable<u64> for u64 {
	fn type_code() -> u32 {
		B_UINT64_TYPE
	}

	fn flattened_size(&self) -> usize {
		8
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<u64, [u8; 8]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<u64> {
		if buffer.len() != 8 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			Ok(buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u64))
		}
	}
}

impl Flattenable<f32> for f32 {
	fn type_code() -> u32 {
		B_FLOAT_TYPE
	}

	fn flattened_size(&self) -> usize {
		4
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<f32, [u8; 4]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<f32> {
		if buffer.len() != 4 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			let tmp: u32 = buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u32);
			let tmp: f32 = unsafe { mem::transmute::<u32, f32>(tmp) };
			Ok(tmp)
		}
	}
}

impl Flattenable<f64> for f64 {
	fn type_code() -> u32 {
		B_DOUBLE_TYPE
	}

	fn flattened_size(&self) -> usize {
		8
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flatten(&self) -> Vec<u8> {
		let data = unsafe { mem::transmute::<f64, [u8; 8]>(*self) };
		data.to_vec()
	}

	fn unflatten(buffer: &[u8]) -> Result<f64> {
		if buffer.len() != 8 {
			Err(HaikuError::from(ErrorKind::InvalidData))
		} else {
			let tmp: u64 = buffer.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u64);
			let tmp: f64 = unsafe { mem::transmute::<u64, f64>(tmp) };
			Ok(tmp)
		}
	}
}

impl Flattenable<String> for String {
	fn type_code() -> u32 {
		B_STRING_TYPE
	}

	fn flattened_size(&self) -> usize {
		self.as_bytes().len() + 1 // The C-String will have an additional \0
	}

	fn is_fixed_size() -> bool {
		false
	}

	fn flatten(&self) -> Vec<u8> {
		let data = CString::new(self.clone()).unwrap();
		data.into_bytes_with_nul()
	}

	fn unflatten(buffer: &[u8]) -> Result<String> {
		let s = match CStr::from_bytes_with_nul(buffer) {
			Ok(s) => s,
			Err(e) => return Err(HaikuError::new(ErrorKind::InvalidData, format!("{}", e))),
		};
		let s_vec = s.to_bytes().to_vec();
		match String::from_utf8(s_vec) {
			Ok(s) => Ok(s),
			Err(_) => Err(HaikuError::new(
				ErrorKind::InvalidData,
				"Invalid UTF8 characters",
			)),
		}
	}
}

#[test]
fn test_flattenable_primitives() {
	let value: u8 = 150;
	let flattened_value = value.flatten();
	assert_eq!(flattened_value.len(), value.flattened_size());
	assert_eq!(value, flattened_value[0]);

	let value: i64 = -3_223_372_036_854_775_807;
	let flattened_value = value.flatten();
	let unflattened_value = i64::unflatten(&flattened_value).unwrap();
	assert_eq!(value, unflattened_value);

	let value = "This is a test string".to_string();
	let flattened_value = value.flatten();
	let unflattened_value = String::unflatten(&flattened_value).unwrap();
	assert_eq!(value, unflattened_value);
}
