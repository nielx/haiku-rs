//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::{error, fmt, result, str};
use std::ffi::CStr;

use haiku_sys::status_t;
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
}

#[derive(Clone, Copy, Debug)]
pub enum ErrorKind {
	InvalidInput,
	Other,
}

impl ErrorKind {
	pub(crate) fn as_str(&self) -> &'static str {
		match *self {
			ErrorKind::InvalidInput => "invalid input parameter",
			ErrorKind::Other => "other os error",
		}
	}
}

impl HaikuError {
	pub fn new(kind: ErrorKind) -> HaikuError
	{
		HaikuError { repr: Repr::Simple(kind) }
	}
	
	pub fn from_raw_os_error(code: status_t) -> HaikuError {
		HaikuError { repr: Repr::Os(code) }
	}
	
	pub fn raw_os_error(&self) -> Option<status_t> {
		match self.repr {
			Repr::Os(i) => Some(i),
			Repr::Simple(..) => None,
		}
	}
	
	pub fn kind(&self) -> ErrorKind {
		match self.repr {
			Repr::Os(_) => ErrorKind::Other,
			Repr::Simple(e) => e
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
			Repr::Simple(kind) => fmt.debug_tuple("Kind").field(&kind).finish()
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
