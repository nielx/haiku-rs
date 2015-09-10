//
// Copyright 2015, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::mem;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

use kernel::fs_attr::*;
use kernel::type_constants::*;
use kernel::types::{c_int, ssize_t, off_t};

pub enum AttributeContents {
	UInt8(u8),
	Int8(i8),
	UInt16(u16),
	Int16(i16),
	UInt32(u32),
	Int32(i32),
	UInt64(u64),
	Int64(i64),
	Float(f32),
	Double(f64),
	Bool(bool),
	Text(String),
	Mimetype(String),
	Unknown(u32, Vec<u8>),
}

pub struct AttributeDescriptor {
	pub name: String,
	pub size: i64,
	pub raw_attribute_type: u32,
}

enum file_descriptor {
	owned(File),
	borrowed(c_int)
}


pub struct AttributeIterator {	
	dir: *mut DIR,
	file: file_descriptor,
}

impl Drop for AttributeIterator {
	fn drop(&mut self) {
		let _ = unsafe { fs_close_attr_dir(self.dir) };
	}
}

impl Iterator for AttributeIterator {
	type Item = io::Result<AttributeDescriptor>;
	
	fn next(&mut self) -> Option<io::Result<AttributeDescriptor>> {
		let mut ent = unsafe { fs_read_attr_dir(self.dir) };
		if ent as u32 == 0 {
			// Note: in the BeBook it says that an error will be set, even
			// if we reach the end of the directory. This is not true; if we
			// reach the end of the attributes, there will not be an error.
			// So there is no way to verify whether we really reached the end,
			// or whether something else went wrong in the mean time.
			None
		} else {
			let fd = match self.file {
				file_descriptor::owned(ref f) => f.as_raw_fd(),
				file_descriptor::borrowed(ref f) => *f
			};
			let attr_name = unsafe {CStr::from_ptr(fs_get_attr_name(ent))};
			let buf: &[u8] = attr_name.to_bytes();
			let str_buf: String = String::from_utf8(buf.to_vec()).unwrap();
			let mut attr_info_data = unsafe { mem::zeroed() };
			let stat_result = unsafe {fs_stat_attr(fd, attr_name.as_ptr(), &mut attr_info_data)};
			if stat_result as i32 == -1 {
				return Some(Err(io::Error::last_os_error()));
			}
			// Convert the attribute type to our types
			Some(Ok(AttributeDescriptor{name: str_buf, size: attr_info_data.size, raw_attribute_type: attr_info_data.attr_type}))
		}	
	}
}

pub trait AttributeExt {
	fn iter_attributes(&self) -> io::Result<AttributeIterator>;
	fn find_attribute(&self, name: &str) -> io::Result<AttributeDescriptor>;
	fn read_attribute_raw(&self, name: &str, raw_type: u32, pos: off_t) -> io::Result<Vec<u8>>;
	fn write_attribute_raw(&self, name: &str, raw_type: u32, pos: off_t, buffer: &[u8]) -> io::Result<()>;
	fn remove_attribute(&self, name: &str) -> io::Result<()>;
	
	// Higher-level functions
	fn read_attribute(&self, attribute: &AttributeDescriptor) -> io::Result<AttributeContents> {
		let value = self.read_attribute_raw(&attribute.name, attribute.raw_attribute_type, 0);
		if value.is_err() {
			return Err(value.unwrap_err());
		}
		
		let contents = value.unwrap();
		// what about endianness?
		match attribute.raw_attribute_type {
			B_BOOL_TYPE => {
				if contents[0] as u32 == 0 {
					Ok(AttributeContents::Bool(false))
				} else {
					Ok(AttributeContents::Bool(true))
				}
			}
			B_INT8_TYPE => {
				if contents.len() == 1 {
					Ok(AttributeContents::Int8(contents[0] as i8))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type i8"))
				}
			}
			B_INT16_TYPE => {
				if contents.len() == 2 {
					let result = contents.iter().rev().fold(0, |acc, &b| (acc << 8) | b as i16);
					Ok(AttributeContents::Int16(result))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type i16"))
				}
			}
			B_INT32_TYPE => {
				if contents.len() == 4 {
					let result = contents.iter().rev().fold(0, |acc, &b| (acc << 8) | b as i32);
					Ok(AttributeContents::Int32(result))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type i32"))
				}
			}
			B_INT64_TYPE => {
				if contents.len() == 8 {
					let result = contents.iter().rev().fold(0, |acc, &b| (acc << 8) | b as i64);
					Ok(AttributeContents::Int64(result))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type i64"))
				}
			}
			B_UINT8_TYPE => {
				if contents.len() == 1 {
					Ok(AttributeContents::UInt8(contents[0] as u8))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type u8"))
				}
			}
			B_UINT16_TYPE => {
				if contents.len() == 2 {
					let result = contents.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u16);
					Ok(AttributeContents::UInt16(result))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type u16"))
				}
			}
			B_UINT32_TYPE => {
				if contents.len() == 4 {
					let result = contents.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u32);
					Ok(AttributeContents::UInt32(result))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type u32"))
				}
			}
			B_UINT64_TYPE => {
				if contents.len() == 8 {
					let result = contents.iter().rev().fold(0, |acc, &b| (acc << 8) | b as u64);
					Ok(AttributeContents::UInt64(result))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch for type u64"))
				}
			}
			B_STRING_TYPE => {
				if let Ok(text) = String::from_utf8(contents) {
					Ok(AttributeContents::Text(text))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "invalid string data"))
				}
			},
			B_MIME_STRING_TYPE => {
				if let Ok(text) = String::from_utf8(contents) {
					Ok(AttributeContents::Mimetype(text))
				} else {
					Err(io::Error::new(io::ErrorKind::InvalidData, "invalid string data"))
				}
			},
			_ => Ok(AttributeContents::Unknown(attribute.raw_attribute_type, contents))
		}
	}

	fn write_attribute(&self, name: &str, value: &AttributeContents) -> io::Result<()> {
		match *value {
			AttributeContents::Int8(x) => {
				try!(self.write_attribute_raw(name, B_INT8_TYPE, 0, &[x as u8]))
			},
			AttributeContents::Int16(x) => {
				let data = unsafe { mem::transmute::<i16, [u8; 2]>(x) };
				try!(self.write_attribute_raw(name, B_INT16_TYPE, 0, &data))
			},
			AttributeContents::Int32(x) => {
				let data = unsafe { mem::transmute::<i32, [u8; 4]>(x) };
				try!(self.write_attribute_raw(name, B_INT32_TYPE, 0, &data))
			},
			AttributeContents::Int64(x) => {
				let data = unsafe { mem::transmute::<i64, [u8; 8]>(x) };
				try!(self.write_attribute_raw(name, B_INT64_TYPE, 0, &data))
			},
			AttributeContents::UInt8(x) => {
				try!(self.write_attribute_raw(name, B_UINT8_TYPE, 0, &[x]))
			},
			AttributeContents::UInt16(x) => {
				let data = unsafe { mem::transmute::<u16, [u8; 2]>(x) };
				try!(self.write_attribute_raw(name, B_UINT16_TYPE, 0, &data))
			},
			AttributeContents::UInt32(x) => {
				let data = unsafe { mem::transmute::<u32, [u8; 4]>(x) };
				try!(self.write_attribute_raw(name, B_UINT32_TYPE, 0, &data))
			},
			AttributeContents::UInt64(x) => {
				let data = unsafe { mem::transmute::<u64, [u8; 8]>(x) };
				try!(self.write_attribute_raw(name, B_UINT64_TYPE, 0, &data))
			},
			AttributeContents::Float(x) => {
				let data = unsafe { mem::transmute::<f32, [u8; 4]>(x) };
				try!(self.write_attribute_raw(name, B_FLOAT_TYPE, 0, &data))
			},
			AttributeContents::Double(x) => {
				let data = unsafe { mem::transmute::<f64, [u8; 8]>(x) };
				try!(self.write_attribute_raw(name, B_DOUBLE_TYPE, 0, &data))
			},
			AttributeContents::Bool(x) => {
				let data: u8 = if x { 1 } else { 0 };
				try!(self.write_attribute_raw(name, B_DOUBLE_TYPE, 0, &[data]))
			},
			AttributeContents::Text(ref x) => {
				let data = CString::new(x.clone()).unwrap();
				try!(self.write_attribute_raw(name, B_STRING_TYPE, 0, data.to_bytes()))
			},
			AttributeContents::Mimetype(ref x) => {
				let data = CString::new(x.clone()).unwrap();
				try!(self.write_attribute_raw(name, B_MIME_STRING_TYPE, 0, data.to_bytes()))
			},
			AttributeContents::Unknown(t, ref data) => {
				try!(self.write_attribute_raw(name, t, 0, data))
			},			
		}
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
			Ok(AttributeIterator{dir: d, file: file_descriptor::borrowed(fd)})
		}
	}
	
	fn find_attribute(&self, name: &str) -> io::Result<AttributeDescriptor> {
		let fd = self.as_raw_fd();
		let mut attr_info_data = unsafe { mem::zeroed() };
		let attr_name = CString::new(name).unwrap();
		let stat_result = unsafe {fs_stat_attr(fd, attr_name.as_ptr(), &mut attr_info_data)};
		if stat_result as i32 == -1 {
			return Err(io::Error::last_os_error());
		}
		Ok(AttributeDescriptor{name: name.to_string(), size: attr_info_data.size, raw_attribute_type: attr_info_data.attr_type})
	}
	
	fn read_attribute_raw(&self, name: &str, raw_type: u32, pos: off_t) -> io::Result<Vec<u8>>{
		let fd = self.as_raw_fd();

		// Get attribute stat
		let descriptor = try!(self.find_attribute(name));
		
		// Read the data
		let attr_name = CString::new(descriptor.name).unwrap();
		let mut dst = Vec::with_capacity(descriptor.size as usize);
		let read_size = unsafe { fs_read_attr(fd, attr_name.as_ptr(), descriptor.raw_attribute_type,
												0, dst.as_mut_ptr(), descriptor.size as u32) };
		
		if read_size == -1 {
			return Err(io::Error::last_os_error());
		} else if read_size != descriptor.size as ssize_t {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch between attribute size and read size"));
		}
		unsafe { dst.set_len(read_size as usize) };
		Ok(dst)
	}
	
	fn write_attribute_raw(&self, name: &str, raw_type: u32, pos: off_t, buffer: &[u8]) -> io::Result<()> {
		let fd = self.as_raw_fd();
		
		// Write the data
		let attr_name = CString::new(name).unwrap();
		let write_size = unsafe { fs_write_attr(fd, attr_name.as_ptr(), raw_type, pos, buffer.as_ptr(), buffer.len() as u32) };
		
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
		let file = try!(File::open(self));
		let d = unsafe { fs_fopen_attr_dir(file.as_raw_fd()) };
		
		if (d as u32) == 0 {
			return Err(io::Error::last_os_error());
		} else {
			Ok(AttributeIterator{dir: d, file: file_descriptor::owned(file)})
		}
	}
	
	fn find_attribute(&self, name: &str) -> io::Result<AttributeDescriptor> {
		let file = try!(File::open(self));
		file.find_attribute(name)
	}
	
	fn read_attribute_raw(&self, name: &str, raw_type: u32, pos: off_t) -> io::Result<Vec<u8>> {
		let file = try!(File::open(self));
		file.read_attribute_raw(name, raw_type, pos)
	}
	
	fn write_attribute_raw(&self, name: &str, raw_type: u32, pos: off_t, buffer: &[u8]) -> io::Result<()> {
		use std::fs::OpenOptions;
		
		let file = try!(OpenOptions::new().write(true).open(self));
		file.write_attribute_raw(name, raw_type, pos, buffer)
	}
	
	fn remove_attribute(&self, name: &str) -> io::Result<()> {
		use std::fs::OpenOptions;
		
		let file = try!(OpenOptions::new().write(true).open(self));
		file.remove_attribute(name)
	}
}


#[test]
fn test_attribute_ext() {
	use std::path::Path;
	
	let path = Path::new("/boot/system/apps/StyledEdit");
	let file = File::open(&path).unwrap();
	let attribute_iterator = file.iter_attributes().unwrap();
	for x in attribute_iterator {
		if let Ok(attribute) = x {
			println!("{}: type {}", attribute.name, attribute.raw_attribute_type);
		} else {
			println!("Breaking loop because of error");
			break;
		}
	}
	
	let attribute_data_raw = file.read_attribute_raw("SYS:NAME", 0, 0).unwrap();
	let attribute_data = String::from_utf8(attribute_data_raw).unwrap();
	println!("SYS:NAME for StyledEdit: {}", attribute_data);
}
