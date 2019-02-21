//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::{error, fmt, result, str};
use std::ffi::CStr;

use haiku_sys::status_t;
use haiku_sys::errors::*;
use libc::{c_int, c_char, size_t};

pub type Result<T> = result::Result<T, HaikuError>;

pub struct HaikuError {
	repr: Repr,
}

impl fmt::Debug for HaikuError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.repr, f)
	}
}

enum Repr{
	Os(status_t),
	Simple(ErrorKind),
	Custom(Box<Custom>)
}

#[derive(Debug)]
struct Custom {
	kind: ErrorKind,
	error: Box<dyn error::Error+Send+Sync>,
}

#[derive(Clone, Copy, Debug)]
pub enum ErrorKind {
	InvalidData,
	InvalidInput,
	NotFound,
	Other,
}

impl ErrorKind {
	pub(crate) fn as_str(&self) -> &'static str {
		match *self {
			ErrorKind::InvalidData => "invalid data",
			ErrorKind::InvalidInput => "invalid input parameter",
			ErrorKind::NotFound => "entity not found",
			ErrorKind::Other => "other os error",
		}
	}
}

impl From<ErrorKind> for HaikuError {
	fn from(kind: ErrorKind) -> HaikuError {
		HaikuError {
			repr: Repr::Simple(kind)
		}
	}
}

impl HaikuError {
	pub fn new<E>(kind: ErrorKind, error: E) -> HaikuError
		where E: Into<Box<dyn error::Error+Send+Sync>>
	{
		Self::_new(kind, error.into())
	}
	
	fn _new(kind: ErrorKind, error: Box<dyn error::Error+Send+Sync>) -> HaikuError
	{
		HaikuError {
			repr: Repr::Custom(Box::new(Custom {
				kind,
				error,
			}))
		}
	}
	
	pub fn last_os_error() -> HaikuError {
		// Get the last OS Error
		extern {
			fn _errnop() -> *mut c_int;
		}
		let error = unsafe { *_errnop() as i32 };
		HaikuError::from_raw_os_error(error)
	}

	pub fn from_raw_os_error(code: status_t) -> HaikuError {
		HaikuError { repr: Repr::Os(code) }
	}
	
	pub fn raw_os_error(&self) -> Option<status_t> {
		match self.repr {
			Repr::Os(i) => Some(i),
			Repr::Simple(..) => None,
			Repr::Custom(_) => None
		}
	}
	
	pub fn kind(&self) -> ErrorKind {
		match self.repr {
			Repr::Os(e) => decode_error_kind(e),
			Repr::Simple(e) => e,
			Repr::Custom(ref e) => e.kind
		}
	}
}

impl fmt::Debug for Repr {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Repr::Os(code) => 
				fmt.debug_struct("Os")
					.field("code", &code)
					.field("kind", &ErrorKind::Other)
					.field("message", &"message").finish(),
			Repr::Simple(kind) => fmt.debug_tuple("Kind").field(&kind).finish(),
			Repr::Custom(ref c) => fmt::Debug::fmt(&c, fmt),
		}
	}
}

impl fmt::Display for HaikuError {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		match self.repr {
			Repr::Os(code) => {
				let detail = error_string(code);
				write!(fmt, "{} (os error {})", detail, code)
			}
			Repr::Simple(kind) => write!(fmt, "{}", kind.as_str()),
			Repr::Custom(ref c) => c.error.fmt(fmt),
		}
	}
}

impl error::Error for HaikuError {
	fn description(&self) -> &str {
		self.kind().as_str()
	}
}

// Shamelessly taken from libstd/sys/unix/os.rs
fn error_string(errno: status_t) -> String {
	extern {
		fn strerror_r(errnum: c_int, buf: *mut c_char,
		              buflen: size_t) -> c_int;
	}
	
	let mut buf = [0 as c_char; 128];
	
	let p = buf.as_mut_ptr();
	unsafe {
		if strerror_r(errno as c_int, p, buf.len()) < 0 {
			panic!("strerror_r failure");
		}
		
		let p = p as *const _;
		str::from_utf8(CStr::from_ptr(p).to_bytes()).unwrap().to_owned()
	}
}

fn decode_error_kind(errno: status_t) -> ErrorKind {
	match errno {
		B_BAD_INDEX => ErrorKind::InvalidInput,
		B_BAD_TYPE => ErrorKind::InvalidInput,
		B_BAD_VALUE => ErrorKind::InvalidInput,
		B_MISMATCHED_VALUES => ErrorKind::InvalidInput,
		B_NAME_NOT_FOUND => ErrorKind::NotFound,
		B_NAME_IN_USE => ErrorKind::InvalidInput,
		B_BAD_DATA => ErrorKind::InvalidData,
		B_DONT_DO_THAT => ErrorKind::InvalidInput,
		_ => ErrorKind::Other
	}
}
