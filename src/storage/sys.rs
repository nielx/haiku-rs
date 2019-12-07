//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::mem;
use std::path::Path;

use libc::{dev_t, ino_t, stat};

use ::support::{ErrorKind, HaikuError, Result};

#[repr(C)]
pub(crate) struct entry_ref {
	pub device: dev_t,
	pub directory: ino_t,
	pub name: CString
}

impl entry_ref {
	pub fn from_path(value: &Path) -> Result<Self> {
		// An entry ref requires that the directory exists, but the leaf not
		let directory = match value.parent() {
			Some(path) => path,
			None => return Err(HaikuError::new(ErrorKind::NotFound, "Cannot extract directory for this path")),
		};
		
		let mut directory_stat: stat = unsafe { mem::zeroed() };
		let directory_path = CString::new(directory.as_os_str().as_bytes()).unwrap();
		unsafe {
			if stat(directory_path.as_ptr(), &mut directory_stat) == -1 {
				return Err(HaikuError::last_os_error());
			}
		}
		
		let name = match directory.file_name() {
			Some(n) => CString::new(n.as_bytes()).unwrap(),
			None => return Err(HaikuError::new(ErrorKind::NotFound, "Cannot determine filename for this path")),
		};
				
		Ok(entry_ref{
			device: directory_stat.st_dev,
			directory: directory_stat.st_ino,
			name: name
		})
	}
}

#[test]
fn test_entry_ref_from_path() {
	let path = Path::new("/boot/system/apps/StyledEdit");
	assert!(entry_ref::from_path(&path).is_ok());
	let path = Path::new("/boot/bogus/doesnotexist");
	assert!(entry_ref::from_path(&path).is_err());
}
