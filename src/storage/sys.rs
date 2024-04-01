//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::ffi::CString;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use libc::{dev_t, ino_t, stat};

use haiku_sys::B_REF_TYPE;

use support::{ErrorKind, Flattenable, HaikuError, Result};

#[repr(C)]
pub(crate) struct entry_ref {
	pub device: dev_t,
	pub directory: ino_t,
	pub name: CString,
}

impl entry_ref {
	pub fn from_path(value: &Path) -> Result<Self> {
		// An entry ref requires that the directory exists, but the leaf not
		let directory = match value.parent() {
			Some(path) => path,
			None => {
				return Err(HaikuError::new(
					ErrorKind::NotFound,
					"Cannot extract directory for this path",
				))
			}
		};

		let mut directory_stat: stat = unsafe { mem::zeroed() };
		let directory_path = CString::new(directory.as_os_str().as_bytes()).unwrap();
		unsafe {
			if stat(directory_path.as_ptr(), &mut directory_stat) == -1 {
				return Err(HaikuError::last_os_error());
			}
		}

		let name = match value.file_name() {
			Some(n) => CString::new(n.as_bytes()).unwrap(),
			None => {
				return Err(HaikuError::new(
					ErrorKind::NotFound,
					"Cannot determine filename for this path",
				))
			}
		};

		Ok(entry_ref {
			device: directory_stat.st_dev,
			directory: directory_stat.st_ino,
			name: name,
		})
	}
}

impl Flattenable<entry_ref> for entry_ref {
	fn type_code() -> u32 {
		B_REF_TYPE
	}

	fn flattened_size(&self) -> usize {
		return mem::size_of::<dev_t>()
			+ mem::size_of::<ino_t>()
			+ self.name.as_bytes_with_nul().len();
	}

	fn is_fixed_size() -> bool {
		false
	}

	fn flatten(&self) -> Vec<u8> {
		let mut vec: Vec<u8> = Vec::with_capacity(self.flattened_size());
		vec.extend(self.device.flatten().iter());
		vec.extend(self.directory.flatten().iter());
		vec.extend(self.name.as_bytes_with_nul().iter());
		println!("vec {:?}", vec);
		vec
	}

	fn unflatten(_buffer: &[u8]) -> Result<entry_ref> {
		unimplemented!()
	}
}

#[test]
fn test_entry_ref_from_path() {
	let path = Path::new("/boot/system/apps/StyledEdit");
	assert!(entry_ref::from_path(&path).is_ok());
	let path = Path::new("/boot/bogus/doesnotexist");
	assert!(entry_ref::from_path(&path).is_err());
}
