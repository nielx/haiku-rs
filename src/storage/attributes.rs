//
// Copyright 2018, 2024, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use libc::{
	c_int, c_void, fs_close_attr_dir, fs_fopen_attr_dir, fs_read_attr, fs_read_attr_dir,
	fs_remove_attr, fs_stat_attr, fs_write_attr, off_t, size_t, type_code, DIR,
};

use crate::support::Flattenable;

/// A descriptor with the metadata of an attribute.
pub struct AttributeDescriptor {
	/// The name of the attribute
	pub name: String,
	/// The size of the data on disk
	pub size: i64,
	/// The raw attribute type. This is a unique number that identifies a type.
	pub raw_attribute_type: type_code,
}

enum FileDescriptor {
	Owned(File),
	Borrowed(c_int),
}

/// An iterator to walk through attributes of a file stored on disk.
///
/// The iterator can be acquired through the `AttributeExt::iter_attributes()`
/// method, which is implemented for both `File` and `Path`
pub struct AttributeIterator {
	dir: *mut DIR,
	file: FileDescriptor,
}

impl Drop for AttributeIterator {
	fn drop(&mut self) {
		let _ = unsafe { fs_close_attr_dir(self.dir) };
	}
}

impl Iterator for AttributeIterator {
	type Item = io::Result<AttributeDescriptor>;

	fn next(&mut self) -> Option<io::Result<AttributeDescriptor>> {
		let ent = unsafe { fs_read_attr_dir(self.dir) };
		if ent as u32 == 0 {
			// Note: in the BeBook it says that an error will be set, even
			// if we reach the end of the directory. This is not true; if we
			// reach the end of the attributes, there will not be an error.
			// So there is no way to verify whether we really reached the end,
			// or whether something else went wrong in the mean time.
			None
		} else {
			let fd = match self.file {
				FileDescriptor::Owned(ref f) => f.as_raw_fd(),
				FileDescriptor::Borrowed(ref f) => *f,
			};
			let attr_name = unsafe { (*ent).d_name.as_ptr() };
			let name_str = unsafe { CStr::from_ptr(attr_name) };
			let str_buf: String = name_str.to_string_lossy().into_owned();
			let mut attr_info_data = unsafe { mem::zeroed() };
			let stat_result = unsafe { fs_stat_attr(fd, attr_name, &mut attr_info_data) };
			if stat_result as i32 == -1 {
				return Some(Err(io::Error::last_os_error()));
			}
			// Convert the attribute type to our types
			Some(Ok(AttributeDescriptor {
				name: str_buf,
				size: attr_info_data.size,
				raw_attribute_type: attr_info_data.type_,
			}))
		}
	}
}

/// The `AttributeExt` trait allows for reading attributes on file system objects
///
/// Implementors of this attribute allow you to read file (and directory)
/// attributes that are implemented for Haiku's native BFS. The trait is
/// implemented for both `File` and `Path` objects.
pub trait AttributeExt {
	/// The attribute iterator returns an iterator over all the attributes.
	fn iter_attributes(&self) -> io::Result<AttributeIterator>;

	/// Find an attribute by name
	///
	/// If the attribute cannot be found, an error will be returned.
	fn find_attribute(&self, name: &str) -> io::Result<AttributeDescriptor>;

	/// Read an attribute as a vector of bytes
	///
	/// This method is the low level implementation of the `read_attribute`
	/// method, that is available to read the contents of an attribute into
	/// any type that implements the `Flattenable` interface. It is advised
	/// to use the higher level implementation if you know which Rust type
	/// you want to use.
	///
	/// Note that when you implement this trait for your object, that it is
	/// valid to call this method with size 0. If that's the case, the caller
	/// expects to get the whole attribute, possibly offset by `pos`.
	fn read_attribute_raw(
		&self,
		name: &str,
		raw_type: type_code,
		pos: off_t,
		size: i64,
	) -> io::Result<Vec<u8>>;

	/// Write an attribute from a slice of bytes
	///
	/// This method is the low level implementation of the `write_attribute`
	/// method, that is available to write any type that implements the
	/// `Flattenable` interface as a file system attribute.
	///
	/// Note that this method does not do any check if the data you are
	/// writing is valid for the type you are trying to store.
	/// Therefore it is advisable to use the higher level `write_attribute`
	/// method.
	fn write_attribute_raw(
		&self,
		name: &str,
		raw_type: type_code,
		pos: off_t,
		buffer: &[u8],
	) -> io::Result<()>;

	/// Remove the attribute with the given name
	fn remove_attribute(&self, name: &str) -> io::Result<()>;

	/// Read an attribute and return a Rust object
	///
	/// This method reads the attribute and returns it in the type `T`. Please
	/// note that you should make sure that the type `T` matches the type in the
	/// `AttributeDescriptor`. The type T should implement the Flattenable trait.
	fn read_attribute<T: Flattenable<T>>(&self, attribute: &AttributeDescriptor) -> io::Result<T> {
		let value = self.read_attribute_raw(&attribute.name, attribute.raw_attribute_type, 0, 0);
		if value.is_err() {
			return Err(value.unwrap_err());
		}

		if T::type_code() != attribute.raw_attribute_type {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "type mismatch"));
		}

		let contents = T::unflatten(&value.unwrap());

		match contents {
			Ok(c) => Ok(c),
			Err(_) => Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"error unflattening data",
			)),
		}
	}

	/// Write an object as a file system attribute
	///
	/// This method writes a copy of any object that implements the Flattenable
	/// trait to the file system.
	fn write_attribute<T: Flattenable<T>>(&self, name: &str, value: &T) -> io::Result<()> {
		let data = value.flatten();
		self.write_attribute_raw(name, T::type_code(), 0, &data)?;
		Ok(())
	}
}

impl AttributeExt for File {
	fn iter_attributes(&self) -> io::Result<AttributeIterator> {
		let fd = self.as_raw_fd();
		let d = unsafe { fs_fopen_attr_dir(fd) };

		if (d as u32) == 0 {
			return Err(io::Error::last_os_error());
		} else {
			Ok(AttributeIterator {
				dir: d,
				file: FileDescriptor::Borrowed(fd),
			})
		}
	}

	fn find_attribute(&self, name: &str) -> io::Result<AttributeDescriptor> {
		let fd = self.as_raw_fd();
		let mut attr_info_data = unsafe { mem::zeroed() };
		let attr_name = CString::new(name).unwrap();
		let stat_result = unsafe { fs_stat_attr(fd, attr_name.as_ptr(), &mut attr_info_data) };
		if stat_result as i32 == -1 {
			return Err(io::Error::last_os_error());
		}
		Ok(AttributeDescriptor {
			name: name.to_string(),
			size: attr_info_data.size,
			raw_attribute_type: attr_info_data.type_,
		})
	}

	fn read_attribute_raw(
		&self,
		name: &str,
		_raw_type: u32,
		pos: off_t,
		size: i64,
	) -> io::Result<Vec<u8>> {
		let fd = self.as_raw_fd();

		// Get attribute stat
		let descriptor = self.find_attribute(name)?;

		// Validate input
		if descriptor.size < pos {
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"the position is higher than the size of the attribute",
			));
		}

		// Read the data
		let attr_name = CString::new(descriptor.name).unwrap();
		let len = if size > 0 {
			// Use the user-supplied size
			size
		} else {
			// Calculate the size
			descriptor.size - pos
		};
		let mut dst: Vec<u8> = Vec::with_capacity(descriptor.size as usize);
		let read_size = unsafe {
			fs_read_attr(
				fd,
				attr_name.as_ptr(),
				descriptor.raw_attribute_type,
				pos,
				dst.as_mut_ptr() as *mut c_void,
				len as size_t,
			)
		};

		if read_size == -1 {
			return Err(io::Error::last_os_error());
		}

		unsafe { dst.set_len(read_size as usize) };
		Ok(dst)
	}

	fn write_attribute_raw(
		&self,
		name: &str,
		raw_type: u32,
		pos: off_t,
		buffer: &[u8],
	) -> io::Result<()> {
		let fd = self.as_raw_fd();

		// Write the data
		let attr_name = CString::new(name).unwrap();
		let write_size = unsafe {
			fs_write_attr(
				fd,
				attr_name.as_ptr(),
				raw_type,
				pos,
				buffer.as_ptr() as *const c_void,
				buffer.len() as size_t,
			)
		};

		if write_size < 0 || write_size as usize != buffer.len() {
			return Err(io::Error::last_os_error());
		}
		Ok(())
	}

	fn remove_attribute(&self, name: &str) -> io::Result<()> {
		let fd = self.as_raw_fd();
		let attr_name = CString::new(name).unwrap();
		let result = unsafe { fs_remove_attr(fd, attr_name.as_ptr()) };
		if result == 0 {
			Ok(())
		} else {
			Err(io::Error::last_os_error())
		}
	}
}

impl AttributeExt for Path {
	fn iter_attributes(&self) -> io::Result<AttributeIterator> {
		let file = File::open(self)?;
		let d = unsafe { fs_fopen_attr_dir(file.as_raw_fd()) };

		if (d as u32) == 0 {
			return Err(io::Error::last_os_error());
		} else {
			Ok(AttributeIterator {
				dir: d,
				file: FileDescriptor::Owned(file),
			})
		}
	}

	fn find_attribute(&self, name: &str) -> io::Result<AttributeDescriptor> {
		let file = File::open(self)?;
		file.find_attribute(name)
	}

	fn read_attribute_raw(
		&self,
		name: &str,
		raw_type: u32,
		pos: off_t,
		size: i64,
	) -> io::Result<Vec<u8>> {
		let file = File::open(self)?;
		file.read_attribute_raw(name, raw_type, pos, size)
	}

	fn write_attribute_raw(
		&self,
		name: &str,
		raw_type: u32,
		pos: off_t,
		buffer: &[u8],
	) -> io::Result<()> {
		use std::fs::OpenOptions;

		let file = OpenOptions::new().write(true).open(self)?;
		file.write_attribute_raw(name, raw_type, pos, buffer)
	}

	fn remove_attribute(&self, name: &str) -> io::Result<()> {
		use std::fs::OpenOptions;

		let file = OpenOptions::new().write(true).open(self)?;
		file.remove_attribute(name)
	}
}

#[cfg(test)]
mod test {
	extern crate tempfile;

	use libc::B_STRING_TYPE;
	use std::ffi::CStr;
	use std::fs::File;
	use std::path::Path;

	use crate::storage::attributes::AttributeExt;

	#[test]
	fn test_attribute_ext() {
		// Test the lower and higher level reading api
		let path = Path::new("/boot/system/apps/StyledEdit");
		let file = File::open(&path).unwrap();
		let mut attribute_iterator = file.iter_attributes().unwrap();
		let attribute_descriptor = attribute_iterator
			.find(|attribute| attribute.as_ref().unwrap().name == "SYS:NAME")
			.unwrap();

		let attribute_data_raw = file.read_attribute_raw("SYS:NAME", 0, 0, 0).unwrap();
		let attribute_data_cstring =
			CStr::from_bytes_with_nul(attribute_data_raw.as_slice()).unwrap();
		let attribute_data = attribute_data_cstring.to_str().unwrap();

		let attribute_data_higher_api = file
			.read_attribute::<String>(&attribute_descriptor.unwrap())
			.unwrap();
		assert_eq!(attribute_data, attribute_data_higher_api);

		// Read, write and remove data using the file attribute API
		let temporary_file = tempfile::NamedTempFile::new().unwrap();
		let file = temporary_file.as_file();
		let string_data = String::from("attribute test data");
		let int_data: u8 = 15;
		file.write_attribute("test_string", &string_data).unwrap();
		file.write_attribute("test_u8", &int_data).unwrap();
		let string_read = file
			.read_attribute_raw("test_string", B_STRING_TYPE, 3, 1)
			.unwrap();
		assert_eq!(string_read[0], 'r' as u8);
		let int_attribute = file.find_attribute("test_u8").unwrap();
		let int_read = file.read_attribute::<u8>(&int_attribute).unwrap();
		assert_eq!(int_read, int_data);
		file.remove_attribute("test_u8").unwrap();
		assert!(file.find_attribute("test_u8").is_err());

		// Read, write and remove data using the path attribute API
		let path = temporary_file.path();
		let string_read = path
			.read_attribute_raw("test_string", B_STRING_TYPE, 3, 1)
			.unwrap();
		assert_eq!(string_read[0], 'r' as u8);
		path.write_attribute("test_u8", &int_data).unwrap();
		let int_read = path.read_attribute::<u8>(&int_attribute).unwrap();
		assert_eq!(int_read, int_data);
		path.remove_attribute("test_u8").unwrap();
		assert!(path.find_attribute("test_u8").is_err());
	}
}
