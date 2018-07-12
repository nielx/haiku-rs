//
// Copyright 2015, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]

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
