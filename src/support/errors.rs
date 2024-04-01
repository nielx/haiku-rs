//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::ffi::CStr;
use std::{error, fmt, result, str};

use haiku_sys::errors::*;
use haiku_sys::status_t;
use libc::{c_char, c_int, size_t};

/// This is a shortened version for a standard Rust result that returns a
/// Haiku error.
///
/// It is used throughout the API, except for the storage kit, which reuses
/// the `std::io::Result` type.
pub type Result<T> = result::Result<T, HaikuError>;

/// This struct represents an Error for using this API
///
/// The error is very much based on the standard library's `std::io::Error`,
/// and roughly has the same usage and functionality.
pub struct HaikuError {
	repr: Repr,
}

impl fmt::Debug for HaikuError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.repr, f)
	}
}

enum Repr {
	Os(status_t),
	Simple(ErrorKind),
	Custom(Box<Custom>),
}

#[derive(Debug)]
struct Custom {
	kind: ErrorKind,
	error: Box<dyn error::Error + Send + Sync>,
}

#[derive(Clone, Copy, Debug)]
/// The kind of error that occured
///
/// Note that this list is not complete, there might be more error kinds added
/// in the future.
pub enum ErrorKind {
	/// An operation was (prematurely) interrupted by another system event.
	/// Usually, you can retry the operation in these instances.
	Interrupted,
	/// This error is returned if the function cannot return valid data, for
	/// example due to a system error.
	InvalidData,
	/// This error tells that the user is supplying parameters that are not
	/// valid.
	InvalidInput,
	/// This error is returned when one of the parameters of the function call
	/// refers to something that does not/no longer exists.
	NotFound,
	/// This error is returned when the operation is not allowed, because the
	/// user has insufficient privileges, or the target of the operation does
	/// not allow it.
	NotAllowed,
	/// This error is returned whenever an operation may fail because it times
	/// out.
	TimedOut,
	/// This leftover category is for any other error.
	///
	/// Sometimes a lower level system error is not properly mapped to a higher
	/// level error. This might be corrected in future versions of the crate.
	Other,
}

impl ErrorKind {
	pub(crate) fn as_str(&self) -> &'static str {
		match *self {
			ErrorKind::Interrupted => "interrupted",
			ErrorKind::InvalidData => "invalid data",
			ErrorKind::InvalidInput => "invalid input parameter",
			ErrorKind::NotFound => "entity not found",
			ErrorKind::NotAllowed => "operation not allowed",
			ErrorKind::TimedOut => "operation timed out",
			ErrorKind::Other => "other os error",
		}
	}
}

impl From<ErrorKind> for HaikuError {
	/// This is a shortcut to create a simple error based on an `ErrorKind`.
	fn from(kind: ErrorKind) -> HaikuError {
		HaikuError {
			repr: Repr::Simple(kind),
		}
	}
}

impl HaikuError {
	/// Create a new error with a `kind`, and a custom payload. The most
	/// common use is to attach a `String` that describes the error, but any
	/// struct that implements the `std::error::Error` trait will work.
	pub fn new<E>(kind: ErrorKind, error: E) -> HaikuError
	where
		E: Into<Box<dyn error::Error + Send + Sync>>,
	{
		Self::_new(kind, error.into())
	}

	fn _new(kind: ErrorKind, error: Box<dyn error::Error + Send + Sync>) -> HaikuError {
		HaikuError {
			repr: Repr::Custom(Box::new(Custom { kind, error })),
		}
	}

	/// Create a new error based on the last OS Error.
	///
	/// This function can be used to create an error after calling OS functions
	/// that set the global error number on failure.
	pub fn last_os_error() -> HaikuError {
		// Get the last OS Error
		extern "C" {
			fn _errnop() -> *mut c_int;
		}
		let error = unsafe { *_errnop() as i32 };
		HaikuError::from_raw_os_error(error)
	}

	/// Convert a raw error constant to a `HaikuError` object
	pub fn from_raw_os_error(code: status_t) -> HaikuError {
		HaikuError {
			repr: Repr::Os(code),
		}
	}

	/// Convert the current error into a (lower level) Haiku error constant
	pub fn raw_os_error(&self) -> Option<status_t> {
		match self.repr {
			Repr::Os(i) => Some(i),
			Repr::Simple(..) => None,
			Repr::Custom(_) => None,
		}
	}

	/// Get the `ErrorKind` for the current error
	pub fn kind(&self) -> ErrorKind {
		match self.repr {
			Repr::Os(e) => decode_error_kind(e),
			Repr::Simple(e) => e,
			Repr::Custom(ref e) => e.kind,
		}
	}
}

impl fmt::Debug for Repr {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Repr::Os(code) => fmt
				.debug_struct("Os")
				.field("code", &code)
				.field("kind", &ErrorKind::Other)
				.field("message", &"message")
				.finish(),
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
	extern "C" {
		fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: size_t) -> c_int;
	}

	let mut buf = [0 as c_char; 128];

	let p = buf.as_mut_ptr();
	unsafe {
		if strerror_r(errno as c_int, p, buf.len()) < 0 {
			panic!("strerror_r failure");
		}

		let p = p as *const _;
		str::from_utf8(CStr::from_ptr(p).to_bytes())
			.unwrap()
			.to_owned()
	}
}

fn decode_error_kind(errno: status_t) -> ErrorKind {
	match errno {
		B_BAD_INDEX => ErrorKind::InvalidInput,
		B_BAD_TYPE => ErrorKind::InvalidInput,
		B_BAD_VALUE => ErrorKind::InvalidInput,
		B_INTERRUPTED => ErrorKind::Interrupted,
		B_MISMATCHED_VALUES => ErrorKind::InvalidInput,
		B_NAME_NOT_FOUND => ErrorKind::NotFound,
		B_NAME_IN_USE => ErrorKind::InvalidInput,
		B_BAD_DATA => ErrorKind::InvalidData,
		B_DONT_DO_THAT => ErrorKind::InvalidInput,
		B_NOT_ALLOWED => ErrorKind::NotAllowed,
		B_TIMED_OUT => ErrorKind::TimedOut,
		_ => ErrorKind::Other,
	}
}
