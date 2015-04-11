#![allow(non_camel_case_types)]

pub mod consts {
	pub const B_OS_NAME_LENGTH : usize = 32;
}

pub mod types {
	
	// Copied from libc (not available in the beta channel)
	pub type int32_t = i32;
	pub type uint32_t = u32;
	pub type int64_t = i64;
	pub type c_char = i8;
	pub type size_t = u32;
	pub type ssize_t = i32;
	
	// Haiku default
	pub type area_id = int32_t;
	pub type port_id = int32_t;
	pub type sem_id = int32_t;
	pub type team_id = int32_t;
	pub type thread_id = int32_t;
	
	pub type status_t = int32_t;
	pub type bigtime_t = int64_t;
}

pub mod errors {
	use kernel::types::status_t;
	
	pub const B_OK: status_t = 0;
	pub const B_INTERRUPTED: status_t = 2147483658;
}

pub mod ports {
	use kernel::consts::B_OS_NAME_LENGTH;
	use kernel::types::{c_char, int32_t, uint32_t, size_t, ssize_t, port_id, team_id, status_t, bigtime_t};
	use std::mem;
	
	#[repr(C)]
	#[derive(Copy, Clone)] pub struct port_info {
		pub port: port_id,
		pub team: team_id,
		pub name: [c_char; B_OS_NAME_LENGTH],
		pub capacity: int32_t,
		pub queue_count: int32_t,
		pub total_count: int32_t,
	}

	extern {
		pub fn create_port(capacity: int32_t, name: *const c_char) -> port_id;
		pub fn find_port(name: *const c_char) -> port_id;
		// read_port
		// read_port_etc
		// write_port
		pub fn write_port_etc(port: port_id, code: int32_t, buffer: *const u8,
											bufferSize: size_t, flags: uint32_t,
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
		unsafe { _get_port_info(port, buf, mem::size_of::<port_info>() as u32) }
	}
}

#[test]
fn test_basic_port() {
	use std::ffi::CString;
	use kernel::ports::{port_info, get_port_info};
	use std::mem;
	use std::str;
	
	let port_name = CString::new("test portname").unwrap();
	let port;
	port = unsafe {ports::create_port(16, port_name.as_ptr())};
	let mut portInfo: port_info = unsafe { mem::zeroed() };
	let status = get_port_info(port, &mut portInfo);
	println!("{}, {}, {}", port, status, portInfo.capacity);
}

#[test]
fn test_find_port() {
	use std::ffi::CString;
	use kernel::ports;

	let port_name = CString::new("system:roster").unwrap();
	let roster_port = unsafe{ports::find_port(port_name.as_ptr())};
	println!("{}", roster_port);
}
