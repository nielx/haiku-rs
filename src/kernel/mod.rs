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
	use libc::c_char;
	use haiku_sys::*;
	use std::ffi::{CStr, CString};
	use std::mem;
	use std::time::Duration;
	
	use kernel::teams::Team;
	use support::{ErrorKind, HaikuError, Result};
	
	/// The port object represents a Haiku port
	///
	/// There are two types of ports: there are owned ports, which means that
	/// they are created with this API. An owned port will live as long as the
	/// Port object lives. Owned ports are created with the `Ports::create()`
	/// method. There are also borrowed ports. These are retrieved using
	/// `Ports::find_port()`. These ports will outlive the lifetime of the
	/// `Port` object.
	/// 
	/// In terms of usage safety, ports are very badly designed on Haiku. While
	/// a port does have an owning team, this merely means the port is deleted
	/// when the team is. It does not give any additional privileges. This
	/// means that anyone can read from every port, and even delete every port.
	///
	/// This API makes the assumption that there is one owner of a port. This
	/// owner can read from the port, and has control over closing it. Other
	/// actors should not (and cannot). This means that reading from a port is
	/// only possible if you own it. If you try to read from a port that you
	/// do not own, the library will panic. You do not need to be the owner to
	/// write to a port.
	pub struct Port {
		port: port_id,
		owned: bool
	}
	
	/// Properties of the port
	pub struct PortInfo {
		/// Representation of the team that the port is part of
		pub team: Team,
		/// The name of the port
		pub name: String,
		/// The capacity of the port
		pub capacity: i32,
		/// The number of items in the queue at the time of getting the info
		pub queue_count: i32,
		/// The total number of messages passed through this port
		pub total_count: i32
	}
	
	impl Port {
		/// Create a new port and take ownership of it
		///
		/// This method creates a new port and takes ownership of that port.
		/// The `name` parameter should be no more than 32 characters. The
		/// `capacity` should be zero or higher. On success you will get a new
		/// port object.
		pub fn create(name: &str, capacity: i32) -> Result<Port> {
			if name.len() > B_OS_NAME_LENGTH {
				return Err(HaikuError::new(ErrorKind::InvalidInput));
			}
			let c_name = CString::new(name).unwrap();
			let port = unsafe { create_port(capacity, c_name.as_ptr()) };
			if port < 0 {
				Err(HaikuError::from_raw_os_error(port))
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

		/// Construct a borrowed port from id
		///
		/// If the port exists, this function will return a borrowed `Port`
		/// object. This means that the port will not be deleted when the
		/// object goes out of scope.
		pub fn from_id(id: port_id) -> Option<Port> {
			if id < 0 {
				// Or should we panic?
				return None;
			}
			let mut info: port_info = unsafe { mem::zeroed() };
			let status = unsafe {
				get_port_info(id, &mut info)
			};
			if status == 0 {
				Some(Port {
					port: id,
					owned: false
				})
			} else {
				None
			}
		}

		/// Write data to the port
		///
		/// The data is identified by a `type_code` and is sent as an array of
		/// bytes. If the port has already reached its maximum capacity, this
		/// operation will block until the message can be written.
		pub fn write(&self, type_code: i32, data: &[u8]) -> Result<()>{
			let status = unsafe { 
				write_port(self.port, type_code, data.as_ptr(), data.len() as usize) 
			};
			// TODO: replace with B_OK
			if status == 0 {
				Ok(())
			} else {
				Err(HaikuError::from_raw_os_error(status))
			}
		}
		
		/// Attempt to write data to the port
		///
		/// The data is identified by a `type_code` and is sent as an array of
		/// bytes. If the port has already reached its maximum capacity, this
		/// operation will block until the message can be written, or until the
		/// timeout is reached. Set the timeout to 0 if you want to return
		/// immediately if the port is at capacity.
		pub fn try_write(&self, type_code: i32, data: &[u8], timeout: Duration) -> Result<()>{
			let timeout_ms = timeout.as_secs() as i64 * 1_000_000 + timeout.subsec_micros() as i64;
			let status = unsafe {
				write_port_etc(self.port, type_code, data.as_ptr(), data.len() as usize,
								B_TIMEOUT, timeout_ms) 
			};
			
			// TODO: replace with B_OK
			if status == 0 {
				Ok(())
			} else {
				Err(HaikuError::from_raw_os_error(status))
			}
		}
		
		/// Read data from a port
		///
		/// This method reads the next message from the port. The data is 
		/// returned as a tuple of a type code and a buffer. The method waits
		/// until there is a next message.
		pub fn read(&self) -> Result<((i32, Vec<u8>))> {
			if !self.owned {
				panic!("You are trying to read from a port that you do not own. This is not allowed");
			}
			let size = unsafe { port_buffer_size(self.port) };
			if size < 0 {
				return Err(HaikuError::from_raw_os_error(size as i32));
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
				Err(HaikuError::from_raw_os_error(dst_len as i32))
			} else {
				unsafe { dst.set_len(dst_len as usize); };
				Ok((type_code, dst))
			}
		}
		
		/// Attempt to read data from a port
		///
		/// This method reads the next message from the port. The data is 
		/// returned as a tuple of a type code and a buffer. The method waits
		/// until there is a next message, or until when a timeout if reached.
		/// If you don't want to wait for a message to come in, you can set the
		/// timeout to 0
		pub fn try_read(&self, timeout: Duration) -> Result<((i32, Vec<u8>))> {
			if !self.owned {
				panic!("You are trying to read from a port that you do not own. This is not allowed");
			}
			let timeout_ms = timeout.as_secs() as i64 * 1_000_000 + timeout.subsec_micros() as i64;
			let size = unsafe { 
				port_buffer_size_etc(self.port, B_TIMEOUT, timeout_ms) 
			};
			if size < 0 {
				return Err(HaikuError::from_raw_os_error(size as i32));
			}
			let mut dst = Vec::with_capacity(size as usize);
			let pdst = dst.as_mut_ptr();
			let mut type_code: i32 = 0;
			let dst_len = unsafe {
				// Technically if there is only one consumer of the port, we
				// could use read_port without a timeout, because we already
				// checked if there is a message waiting with a timeout above.
				// However, there might be bad actors out there that are also
				// listening to this port, so using the timeout again will
				// prevent a lock when that's the case.
				read_port_etc(self.port, &mut type_code, pdst, size as usize,
				              B_TIMEOUT, timeout_ms)
			};
			
			if dst_len > 0 && dst_len != size {
				panic!("read_port does not return data with the predicted size");
			}
			
			if dst_len < 0 {
				Err(HaikuError::from_raw_os_error(dst_len as i32))
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
		pub fn close(&self) -> Result<()> {
			if !self.owned {
				panic!("You are trying to close a port that you do not own. This is not allowed");
			}

			let status = unsafe { close_port(self.port) };
			if status == 0 {
				Ok(())
			} else {
				Err(HaikuError::from_raw_os_error(status))
			}
		}
		
		/// Get the port info
		pub fn get_info(&self) -> Result<PortInfo> {
			let mut info: port_info = unsafe { mem::zeroed() };
			let status = unsafe {
				get_port_info(self.port, &mut info)
			};
			if status != 0 {
				Err(HaikuError::from_raw_os_error(status))
			} else {
				let c_name = unsafe {
					CStr::from_ptr((&info.name) as *const c_char)
				};
				Ok(PortInfo{
					team: Team::from(info.team).unwrap(),
					name: String::from(c_name.to_str().unwrap()),
					capacity: info.capacity,
					queue_count: info.queue_count,
					total_count: info.total_count
				})
			}
		}
		
		/// Get the underlying port id
		pub fn get_port_id(&self) -> port_id{
			self.port
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


/// A team is a unique process that is running on Haiku
pub mod teams {
	use haiku_sys::*;
	/// This struct is a representation of a team
	pub struct Team {
		id: team_id
	}
	
	impl Team {
		/// Build a team object from a raw team id
		pub fn from(id: team_id) -> Option<Team> {
			if id < 0 {
				None
			} else {
				Some(Team{ id })
			}
		}
		
		/// Get the raw team identifier
		pub fn get_team_id(&self) -> team_id {
			self.id
		}
	}
}

// Helpers for this crate only
pub(crate) mod helpers {
	use std::ffi::CStr;
	use std::str;
	
	use haiku_sys::*;
	use libc::{c_char, dev_t, ino_t, size_t}; 
	
	use support::{Result, HaikuError};
	
	pub(crate) fn get_path_for_entry_ref(device: dev_t, dir: ino_t, leaf: *const c_char) -> Result<String> {
		extern {
			pub fn _kern_entry_ref_to_path(device: dev_t, inode: ino_t, leaf: *const c_char, buf: *mut c_char, bufferSize: size_t) -> status_t;
		}
		
		let mut buf = [0 as c_char; B_PATH_NAME_LENGTH];
		let p = buf.as_mut_ptr();
		let path = unsafe {
			let result = _kern_entry_ref_to_path(device, dir, leaf, p, buf.len());
			if result != 0 {
				return Err(HaikuError::from_raw_os_error(result));
			}
			
			let p = p as *const _;
			str::from_utf8(CStr::from_ptr(p).to_bytes()).unwrap().to_owned()
		};
		Ok(path)
	}
}
				


// Todo: legacy code, this should be moved to haiku-sys
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
fn test_port_with_timeout() {
	use kernel::ports::Port;
	use std::time::Duration;
	
	let port = Port::create("timeout_port", 1).unwrap();
	assert!(port.try_read(Duration::new(5,0)).is_err());
}

#[test]
fn test_find_port() {
	use kernel::ports::Port;
	assert!(Port::find("x-vnd.haiku-debug_server").is_some());
	assert!(Port::find("random port").is_none());
}
