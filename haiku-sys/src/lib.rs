//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]

extern crate libc;
use libc::{c_int, c_char, DIR, dirent, off_t, size_t, ssize_t};


// OS.h
pub type area_id = i32;
pub type port_id = i32;
pub type sem_id = i32;
pub type team_id = i32;
pub type thread_id = i32;

pub type status_t = i32;
pub type bigtime_t = i64;

// fs_attr.h
#[repr(C)]
pub struct attr_info {
	pub attr_type: u32,
	pub size: off_t,
}

extern {
	pub fn fs_read_attr(fd: c_int, attribute: *const c_char, typeCode: u32,
						pos: off_t, buffer: *mut u8, readBytes: size_t) -> ssize_t;
	pub fn fs_write_attr(fd: c_int, attribute: *const c_char, typeCode: u32,
						pos: off_t, buffer: *const u8, readBytes: size_t) -> ssize_t;
	pub fn fs_remove_attr(fd: c_int, attribute: *const c_char) -> c_int;
	pub fn fs_stat_attr(fd: c_int, attribute: *const c_char, attrInfo: *mut attr_info) -> c_int;
	
	pub fn fs_open_attr(path: *const c_char, attribute: *const c_char,
						typeCode: u32, openMode: c_int) -> c_int;
	pub fn fs_fopen_attr(fd: c_int, attribute: *const c_char, typeCode: u32, 
						openMode: c_int) -> c_int;
	pub fn fs_close_attr(fd: c_int) -> c_int;
	
	pub fn fs_open_attr_dir(path: *const c_char) -> *mut DIR;
	pub fn fs_lopen_attr_dir(path: *const c_char) -> *mut DIR;
	pub fn fs_fopen_attr_dir(fd: c_int) -> *mut DIR;
	pub fn fs_close_attr_dir(dir: *mut DIR) -> c_int;
	pub fn fs_read_attr_dir(dir: *mut DIR) -> *mut dirent;
	pub fn fs_rewind_attr_dir(dir: *mut DIR);
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
