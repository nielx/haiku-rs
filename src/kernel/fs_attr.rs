
use kernel::types::{c_char, c_int, off_t, uint32_t, size_t, ssize_t};

// Copied from libc, which is not available in the beta channel
pub enum DIR {}
pub enum dirent_t {}

#[repr(C)]
#[derive(Copy, Clone)] pub struct attr_info {
	pub attr_type: uint32_t,
	pub size: off_t,
}

extern {
	pub fn fs_read_attr(fd: c_int, attribute: *const c_char, typeCode: uint32_t,
						pos: off_t, buffer: *mut u8, readBytes: size_t) -> ssize_t;
	pub fn fs_write_attr(fd: c_int, attribute: *const c_char, typeCode: uint32_t,
						pos: off_t, buffer: *const u8, readBytes: size_t) -> ssize_t;
	pub fn fs_remove_attr(fd: c_int, attribute: *const c_char) -> c_int;
	pub fn fs_stat_attr(fd: c_int, attribute: *const c_char, attrInfo: *mut attr_info) -> c_int;
	
	pub fn fs_open_attr(path: *const c_char, attribute: *const c_char,
						typeCode: uint32_t, openMode: c_int) -> c_int;
	pub fn fs_fopen_attr(fd: c_int, attribute: *const c_char, typeCode: uint32_t, 
						openMode: c_int) -> c_int;
	pub fn fs_close_attr(fd: c_int) -> c_int;
	
	pub fn fs_open_attr_dir(path: *const c_char) -> *mut DIR;
	pub fn fs_lopen_attr_dir(path: *const c_char) -> *mut DIR;
	pub fn fs_fopen_attr_dir(fd: c_int) -> *mut DIR;
	pub fn fs_close_attr_dir(dir: *mut DIR) -> c_int;
	pub fn fs_read_attr_dir(dir: *mut DIR) -> *mut dirent_t;
	pub fn fs_rewind_attr_dir(dir: *mut DIR);
}

pub unsafe fn fs_get_attr_name(dirent: *mut dirent_t) -> *const c_char {
	extern {
		fn rust_list_dir_val(ptr: *mut dirent_t) -> *const c_char;
	}
	return rust_list_dir_val(dirent);
}



#[test]
fn test_fs_attr() {
	use std::ffi::CStr;
	use std::ffi::CString;
	use std::fs::File;
	use std::mem;
	use std::os::unix::io::AsRawFd;
	use std::ptr;
	use std::str;
	
	use kernel::type_constants::*;
	
	let target_file = "/boot/system/apps/StyledEdit";
	
	let file_name = CString::new(target_file).unwrap();
	let d = unsafe { fs_open_attr_dir(file_name.as_ptr()) };
	assert!(d != 0 as *mut DIR);
	
	let file = File::open(target_file).unwrap();
	let mut ent = unsafe{ fs_read_attr_dir(d) };
	while ent as u32 != 0 {
		let attr_name = unsafe {CStr::from_ptr( fs_get_attr_name(ent) ) };
		println!( "{:?}", str::from_utf8(attr_name.to_bytes()).unwrap() );
		
		// get the stat
		unsafe {
			let mut attr_info_data: attr_info = mem::zeroed();
			let result = fs_stat_attr(file.as_raw_fd(), attr_name.as_ptr(), &mut attr_info_data);
			assert!(result == 0);
			
			match attr_info_data.attr_type {
				B_STRING_TYPE => println!("\tText type"),
				B_MIME_STRING_TYPE => println!("\tMime string"),
				_ => println!("\tUnknown type")
			}
		}
		
		ent = unsafe { fs_read_attr_dir(d) };
	}
	unsafe { fs_close_attr_dir(d) };
}
