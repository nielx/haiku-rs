//
// Copyright 2015, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#[cfg(test)]
mod tests {
	use std::io::prelude::*;
	
	use std::env;
	use std::ffi::CString;
	use std::fs;
	use std::fs::File;
	use std::mem;
	use std::path::PathBuf;
	use std::path::Path as Path2;
	use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

	use kernel::file_open_mode_constants::{B_CREATE_FILE, B_READ_ONLY, B_WRITE_ONLY};
	use kernel::fs_attr::*;
	use kernel::type_constants::*;
	use kernel::types::{c_char, c_int, size_t, ssize_t};
	
	macro_rules! check { ($e:expr) => (
		match $e {
			Ok(t) => t,
			Err(e) => panic!("{} failed with: {}", stringify!($e), e),
		}
	) }
	
	extern {
		pub fn rand() -> c_int;
	}
	
	// TempDir inspired by the libstd test for fs operations
	pub struct TempDir(PathBuf);
	
	impl TempDir {
		fn join(&self, path: &str) -> PathBuf {
			let TempDir(ref p) = *self;
			p.join(path)
		}
		
		fn path<'a>(&'a self) -> &'a Path2 {
			let TempDir(ref p) = *self;
			p
		}
	}
	
	impl Drop for TempDir {
		fn drop(&mut self) {
			let TempDir(ref p) = *self;
			//check!(fs::remove_dir_all(p));
		}
	}
	
	fn tmpdir() -> TempDir {
		let p = env::temp_dir();
		let r = unsafe { rand() }; // TODO: use srand
		let ret = p.join(&format!("haiku-rs-{}", r));
		check!(fs::create_dir(&ret));
		TempDir(ret)
	}
	
	fn count_attributes(fd: c_int) -> usize {
		let mut count: usize = 0;
		let d = unsafe { fs_fopen_attr_dir(fd) };
		assert!(d != 0 as *mut DIR);
		let mut ent = unsafe{ fs_read_attr_dir(d) };
		while ent as u32 != 0 {
			count += 1;
			ent = unsafe { fs_read_attr_dir(d) };
		}
		let close = unsafe { fs_close_attr_dir(d) };
		assert_eq!(close, 0);
		count
	}
	
	fn stat_attribute(fd: c_int, attribute: *const c_char) -> attr_info {
		let mut attr_info_data = unsafe { mem::zeroed() };
		let stat_result = unsafe {fs_stat_attr(fd, attribute, &mut attr_info_data)};
		assert!(stat_result != -1);
		attr_info_data
	}
	
	#[test]
	fn test_fs_attr() {
		let tmpdir = tmpdir();
		let filename = &tmpdir.join("file_attributes_test.txt");
		{
			// create the file
			let mut file = check!(File::create(filename));
			file.write("bogus data".as_bytes());
		}
		let mut file = check!(File::open(filename));
		assert_eq!(count_attributes(file.as_raw_fd()), 0);
		
		// Add three attributes
		let attr_1: u8 = 147;
		let attr_2 = CString::new("A normal string").unwrap();
		let attr_3 = CString::new("application/x-vnd.rust-test").unwrap();
		let attr_1_name = CString::new("ATTRIBUTE_ONE").unwrap();
		let attr_2_name = CString::new("ATTRIBUTE_TWO").unwrap();
		let attr_3_name = CString::new("ATTRIBUTE_THREE").unwrap();
		
		unsafe {
			let mut result: ssize_t;
			result = fs_write_attr(file.as_raw_fd(), attr_1_name.as_ptr(),
			                      B_UINT8_TYPE, 0, &attr_1, 1);
			assert_eq!(result, 1);
			result = fs_write_attr(file.as_raw_fd(), attr_2_name.as_ptr(),
			                      B_STRING_TYPE, 0, attr_2.as_ptr() as *const u8, 
			                      attr_2.as_bytes_with_nul().len() as u32);
			assert_eq!(result, attr_2.as_bytes_with_nul().len() as i32);
			
			// Use the other interface here
			let attr_fd = fs_fopen_attr(file.as_raw_fd(),
			                            attr_3_name.as_ptr(), B_MIME_STRING_TYPE, 
			                            B_WRITE_ONLY | B_CREATE_FILE);
			assert!(attr_fd > 0);
			let mut attr_file = File::from_raw_fd(attr_fd);
			result = check!(attr_file.write(attr_3.as_bytes_with_nul())) as ssize_t;
			// We don't need to close the file descriptor; Rust will do that
			// This means we don't test fs_close_attr(), but that's ok since it
			// is just a wrapper around the standard close anyway.
			
			assert_eq!(result, attr_3.as_bytes_with_nul().len() as i32);
			assert_eq!(count_attributes(file.as_raw_fd()), 3);
		}
		
		// Read back the three attributes
		unsafe {
			let mut result: ssize_t;
			let mut buffer: Vec<u8>;
			let mut stat: attr_info;
			let mut size: size_t;
			
			size = 1;
			stat = stat_attribute(file.as_raw_fd(), attr_1_name.as_ptr());
			assert_eq!(stat.attr_type, B_UINT8_TYPE);
			assert_eq!(stat.size, 1);
			
			buffer = Vec::with_capacity(1 as usize);
			result = fs_read_attr(file.as_raw_fd(), attr_1_name.as_ptr(),
			                     B_UINT8_TYPE, 0, buffer.as_mut_ptr(), 1);
			assert_eq!(result, size as ssize_t);
			buffer.set_len(size as usize);
			assert_eq!(buffer[0], attr_1);
			
			// skip attribute 2, do that one below
			size = attr_3.as_bytes_with_nul().len() as size_t;
			stat = stat_attribute(file.as_raw_fd(), attr_3_name.as_ptr());
			assert_eq!(stat.attr_type, B_MIME_STRING_TYPE);
			assert_eq!(stat.size, size as i64);
			
			buffer = Vec::with_capacity(size as usize);
			result = fs_read_attr(file.as_raw_fd(), attr_3_name.as_ptr(),
			                     B_MIME_STRING_TYPE, 0, buffer.as_mut_ptr(), size);
			 
			assert_eq!(result, size as ssize_t);
			buffer.set_len(size as usize);
			assert_eq!(buffer, attr_3.as_bytes_with_nul());
			
			// use the other api to read attribute 2
			size = attr_2.as_bytes_with_nul().len() as size_t;
			stat = stat_attribute(file.as_raw_fd(), attr_2_name.as_ptr());
			assert_eq!(stat.attr_type, B_STRING_TYPE);
			assert_eq!(stat.size, size as i64);
			
			buffer = Vec::with_capacity(size as usize);
			let attr_fd = fs_fopen_attr(file.as_raw_fd(),
			                            attr_2_name.as_ptr(), B_STRING_TYPE, 
			                            B_READ_ONLY);
			assert!(attr_fd > 0);
			let mut attr_file = File::from_raw_fd(attr_fd);
			buffer.set_len(size as usize);
			result = check!(attr_file.read(&mut buffer)) as ssize_t;
			assert_eq!(result, size as ssize_t);
			assert_eq!(buffer, attr_2.as_bytes_with_nul());
		}
		
		// Remove an attribute
		unsafe {
			let result = fs_remove_attr(file.as_raw_fd(), attr_3_name.as_ptr());
			assert_eq!(result, 0);
			assert_eq!(count_attributes(file.as_raw_fd()), 2)
		}
	}
}
