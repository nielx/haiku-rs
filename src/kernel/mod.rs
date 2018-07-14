//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]

/// A port is a system-wide communication channel that can be used to copy
/// data between threads and teams.
///
/// Ports are the lower level transportation mechanism for Messages. 
pub mod ports {
	use haiku_sys::*;
	use std::io;
	use std::io::{Error, ErrorKind};
	use std::ffi::CString;
	
	/// The port object represents a Haiku port
	///
	/// There are two types of ports: there are owned ports, which means that
	/// they are created with this API. An owned port will live as long as the
	/// Port object lives. Owned ports are created with the `Ports::create()`
	/// method. There are also borrowed ports. These are retrieved using
	/// `Ports::find_port()`. These ports will outlive the lifetime of the
	/// `Port` object.
	pub struct Port {
		port: port_id,
		owned: bool
	}
	
	impl Port {
		/// Create a new port and take ownership of it
		///
		/// This method creates a new port and takes ownership of that port.
		/// The `name` parameter should be no more than 32 characters. The
		/// `capacity` should be zero or higher. On success you will get a new
		/// port object.
		pub fn create(name: &str, capacity: i32) -> io::Result<Port> {
			if name.len() > B_OS_NAME_LENGTH {
				return Err(Error::new(ErrorKind::InvalidInput, "the name is too large"));
			}
			let c_name = CString::new(name).unwrap();
			let port = unsafe { create_port(capacity, c_name.as_ptr()) };
			if port < 0 {
				Err(Error::from_raw_os_error(port))
			} else {
				Ok(Port {
					port: port,
					owned: true
				})
			}
		}
		
		/// Find an existing port by name
		///
		/// If the port exists, this function will return a borrowed `Port`
		/// object. This means that the port will not be deleted when the
		/// object goes out of scope.
		pub fn find(name: &str) -> Option<Port> {
			if name.len() > B_OS_NAME_LENGTH {
				// Or should we panic?
				return None;
			}
			
			let c_name = CString::new(name).unwrap();
			let port = unsafe { find_port(c_name.as_ptr()) };
			if port < 0 {
				None 
			} else {
				Some(Port {
					port: port,
					owned: false
				})
			}
		}
		
		/// Write data to the port
		///
		/// The data is identified by a `type_code` and is sent as an array of
		/// bytes. If the port has already reached its maximum capacity, this
		/// operation will block until the message can be written.
		pub fn write(&self, type_code: i32, data: &[u8]) -> io::Result<()>{
			let status = unsafe { 
				write_port(self.port, type_code, data.as_ptr(), data.len() as usize) 
			};
			// TODO: replace with B_OK
			if status == 0 {
				Ok(())
			} else {
				Err(Error::from_raw_os_error(status))
			}
		}
		
		/// Read data from a port
		///
		/// This method reads the next message from the port. The data is 
		/// returned as a tuple of a type code and a buffer. The method waits
		/// until there is a next message.
		pub fn read(&self) -> io::Result<((i32, Vec<u8>))> {
			let size = unsafe { port_buffer_size(self.port) };
			if size < 0 {
				return Err(Error::from_raw_os_error(size as i32));
			}
			let mut dst = Vec::with_capacity(size as usize);
			let pdst = dst.as_mut_ptr();
			let mut type_code: i32 = 0;
			let dst_len = unsafe { 
				read_port(self.port, &mut type_code, pdst, size as usize)
			};
			
			if dst_len > 0 && dst_len != size {
				panic!("read_port does not return data with the predicted size");
			}
			
			if dst_len < 0 {
				Err(Error::from_raw_os_error(dst_len as i32))
			} else {
				unsafe { dst.set_len(dst_len as usize); };
				Ok((type_code, dst))
			}
		}
		
		/// Close a port
		///
		/// When a port is closed, data can no longer be written to it. The
		/// message queue can still be read. Once a port is closed, it cannot
		/// be reopened.
		pub fn close(&self) -> io::Result<()> {
			let status = unsafe { close_port(self.port) };
			if status == 0 {
				Ok(())
			} else {
				Err(Error::from_raw_os_error(status))
			}
		}
	}
	
	impl Drop for Port {
		fn drop(&mut self) {
			if self.owned {
				unsafe { delete_port(self.port); };
			}
		}
	}
}
	

// TODO: Legacy code, this should be moved soon!
pub mod errors {
	use haiku_sys::status_t;
	
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
	use kernel::ports::Port;
	
	let port = Port::create("test_basic_port", 16).unwrap();
	let port_data = b"testdata for port\n";
	let port_code: i32 = 47483658;
	port.write(port_code, port_data).unwrap();
	let (read_code, read_data) = port.read().unwrap();
	assert_eq!(port_code, read_code);
	assert_eq!(port_data.len(), read_data.len());
	port.close().unwrap();
	assert!(port.write(port_code, port_data).is_err());
}

#[test]
fn test_find_port() {
	use kernel::ports::Port;
	assert!(Port::find("x-vnd.haiku-debug_server").is_some());
	assert!(Port::find("random port").is_none());
}
