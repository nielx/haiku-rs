
use kernel::types::{c_char, c_int, off_t, uint32_t};

// Copied from libc, which is not available in the beta channel
pub enum DIR {}
pub enum dirent_t {}

#[repr(C)]
#[derive(Copy, Clone)] pub struct attr_info {
	pub attr_type: uint32_t,
	pub size: off_t,
}

extern {
	// fs_read_attr
	// fs_write_attr
	// fs_remove_attr
	pub fn fs_stat_attr(fd: c_int, attribute: *const c_char, attrInfo: *mut attr_info) -> c_int;
	
	// fs_open_attr
	// fs_fopen_attr
	// fs_close_attr
	
	pub fn fs_open_attr_dir(path: *const c_char) -> *mut DIR;
	// fs_lopen_attr_dir
	// fs_fopen_attr_dir
	pub fn fs_close_attr_dir(dir: *mut DIR) -> c_int;
	pub fn fs_read_attr_dir(dir: *mut DIR) -> *mut dirent_t;
	// fs_rewind_attr_dir
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
	use std::ptr;
	use std::str;
	
	let file_name = CString::new("/boot/system/apps/StyledEdit").unwrap();
	let d = unsafe { fs_open_attr_dir(file_name.as_ptr()) };
	assert!(d != 0 as *mut DIR);
	
	//let file = File::open("/boot/system/apps/StyledEdit").unwrap();
	let mut ent = unsafe{ fs_read_attr_dir(d) };
	while ent as u32 != 0 {
		let attr_name = unsafe {CStr::from_ptr( fs_get_attr_name(ent) ) };
		println!( "{:?}", str::from_utf8(attr_name.to_bytes()).unwrap() );
		ent = unsafe { fs_read_attr_dir(d) };
	}
	unsafe { fs_close_attr_dir(d) };
}
