//
// Copyright 2015, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]

pub mod consts {
	pub const B_OS_NAME_LENGTH : usize = 32;
}

pub mod types {
	// Haiku default
	pub type area_id = i32;
	pub type port_id = i32;
	pub type sem_id = i32;
	pub type team_id = i32;
	pub type thread_id = i32;
	
	pub type status_t = i32;
	pub type bigtime_t = i64;
}

pub mod errors {
	use kernel::types::status_t;
	
	pub const B_OK: status_t = 0;
	pub const B_INTERRUPTED: status_t = 2147483658;
}

pub mod type_constants {	
	// not an exhaustive list!
	pub const B_MIME_STRING_TYPE: u32 = 1296649555;
	pub const B_STRING_TYPE: u32 = 1129534546;
	pub const B_BOOL_TYPE: u32 = 1112493900;
	pub const B_DOUBLE_TYPE: u32 = 1145195589;
	pub const B_FLOAT_TYPE: u32 = 1179406164;
	pub const B_INT8_TYPE: u32 = 1113150533;
	pub const B_INT16_TYPE: u32 = 1397248596;
	pub const B_INT32_TYPE: u32 = 1280265799;
	pub const B_INT64_TYPE: u32 = 1280069191;
	pub const B_UINT8_TYPE: u32 = 1430411604;
	pub const B_UINT16_TYPE: u32 = 1431521364;
	pub const B_UINT32_TYPE: u32 = 1431064135;
	pub const B_UINT64_TYPE: u32 = 1431063623;
}

pub mod file_open_mode_constants {
	use libc::c_int;
	
	pub const B_READ_ONLY: c_int = 0x0000;
	pub const B_WRITE_ONLY: c_int = 0x0001;
	pub const B_READ_WRITE: c_int = 0x0002;
	
	pub const B_FAIL_IF_EXISTS: c_int = 0x0100;
	pub const B_CREATE_FILE: c_int = 0x0200;
	pub const B_ERASE_FILE: c_int = 0x0400;
	pub const B_OPEN_AT_END: c_int = 0x0800;
}

pub mod ports {
	use kernel::consts::B_OS_NAME_LENGTH;
	use libc::{c_char, size_t, ssize_t};
	use kernel::types::{port_id, team_id, status_t, bigtime_t};
	use std::mem;
	
	#[repr(C)]
	#[derive(Copy, Clone)] pub struct port_info {
		pub port: port_id,
		pub team: team_id,
		pub name: [c_char; B_OS_NAME_LENGTH],
		pub capacity: i32,
		pub queue_count: i32,
		pub total_count: i32,
	}

	extern {
		pub fn create_port(capacity: i32, name: *const c_char) -> port_id;
		pub fn find_port(name: *const c_char) -> port_id;
		pub fn read_port(port: port_id, code: *mut i32, buffer: *mut u8,
											bufferSize: size_t) -> ssize_t;
		// read_port_etc
		pub fn write_port(port: port_id, code: i32, buffer: *const u8,
											bufferSize: size_t) -> status_t;
		pub fn write_port_etc(port: port_id, code: i32, buffer: *const u8,
											bufferSize: size_t, flags: u32,
											timeout: bigtime_t) -> status_t;
		pub fn close_port(port: port_id) -> status_t;
		pub fn delete_port(port: port_id) -> status_t;
		pub fn port_buffer_size(port: port_id) -> ssize_t;
		// port_buffer_size_etc
		pub fn port_count(port: port_id) -> ssize_t;
		// set_port_owner
		
		fn _get_port_info(port: port_id, buf: *mut port_info,
			              portInfoSize: size_t) -> status_t;
		// _get_next_port_info 
	}
	
	pub fn get_port_info(port: port_id, buf: &mut port_info) -> status_t {
		unsafe { _get_port_info(port, buf, mem::size_of::<port_info>() as size_t) }
	}
}

pub fn debugger(message: &str) {
	use libc::c_char;
	use std::ffi::CString;
	extern {
		fn debugger(message: *const c_char);
	}
	let msg = CString::new(message).unwrap();
	unsafe { debugger(msg.as_ptr()) };
}
	

#[test]
fn test_basic_port() {
	use std::ffi::CString;
	use kernel::errors::{B_OK, B_INTERRUPTED};
	use libc::{size_t, ssize_t};
	use kernel::ports::*;
	use std::mem;
	use std::str;
	
	let port_name = CString::new("test_basic_port").unwrap();
	let port;
	port = unsafe {ports::create_port(16, port_name.as_ptr())};
	assert!(port > 0);
	let mut portInfo: port_info = unsafe { mem::zeroed() };
	let mut status = get_port_info(port, &mut portInfo);
	assert!(status == B_OK);
	assert!(portInfo.port == port);
	assert!(portInfo.capacity == 16);
	
	let port_data = b"testdata for port\n";
	let port_code: i32 = 47483658;
	status = unsafe { write_port(port, port_code, port_data.as_ptr(), port_data.len() as u32) };
	
	// wait for the data to be readable
	let mut size: ssize_t = B_INTERRUPTED;
	while size == B_INTERRUPTED {
		size = unsafe { port_buffer_size(port) };
	}
	assert!(size == port_data.len() as i32);
	
	// read the data
	let mut incoming_data = Vec::with_capacity(size as usize);
	let mut incoming_code: i32 = 0;
	
	let read_size = unsafe { read_port(port, &mut incoming_code, incoming_data.as_mut_ptr(), size as size_t) };
	assert!(read_size == size);
	assert!(incoming_code == port_code);
	
	// close the port
	status = unsafe { close_port(port) };
	assert!(status == B_OK);
	
	// delete the port
	status = unsafe { delete_port(port) };
	assert!(status == B_OK);	
}

#[test]
fn test_find_port() {
	use std::ffi::CString;
	use kernel::ports;

	let port_name = CString::new("system:roster").unwrap();
	let roster_port = unsafe{ports::find_port(port_name.as_ptr())};
	println!("{}", roster_port);
}
