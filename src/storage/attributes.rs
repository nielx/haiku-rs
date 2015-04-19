use std::ffi::CStr;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::str;

use kernel::fs_attr::*;


pub trait AttributeExt {
	fn get_attributes(&self) -> Vec<String>;
}

impl AttributeExt for File {
	fn get_attributes(&self) -> Vec<String> {
		let fd = self.as_raw_fd();
		let d = unsafe { fs_fopen_attr_dir(fd) };
		let mut result: Vec<String> = Vec::new();
		
		let mut ent = unsafe { fs_read_attr_dir(d) };
		while ent as u32 != 0 {
			let attr_name = unsafe {CStr::from_ptr(fs_get_attr_name(ent))};
			let buf: &[u8] = unsafe { attr_name.to_bytes() };
			let str_slice: &str = str::from_utf8(buf).unwrap();
			let str_buf: String = String::from_utf8(buf.to_vec()).unwrap();
			result.push(str_buf);
			ent = unsafe { fs_read_attr_dir(d) };
		}
		
		unsafe { fs_close_attr_dir(d) };
		
		result
	}
}

#[test]
fn test_attribute_ext() {
	use std::path::Path;
	
	let path = Path::new("/boot/system/apps/StyledEdit");
	let file = File::open(&path).unwrap();
	let attributes = file.get_attributes();
	for x in attributes.iter() {
		println!("{}", x);
	}
}
