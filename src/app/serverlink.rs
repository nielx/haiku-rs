//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

// This module contains the private messaging interface between Haiku applications
// and the app server.

use std::env;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::mem;

use haiku_sys::port_id;

use ::app::message::Message;
use ::app::messenger::Messenger;
use ::kernel::ports::Port;
use ::support::{Result, Flattenable, HaikuError, ErrorKind};

#[repr(C)]
struct message_header {
	size: i32,
	code: u32,
	flags: u32
}

const LINK_CODE: i32 = haiku_constant!('_','P','T','L') as i32;
const INITIAL_BUFFER_SIZE: usize = 2048;
const BUFFER_WATERMARK: u64 = INITIAL_BUFFER_SIZE as u64 - 24;
const MAX_BUFFER_SIZE: usize = 65536;
const MAX_STRING_SIZE: usize = 4096;
const NEEDS_REPLY: u32 = 0x01;

#[allow(dead_code)]
pub(crate) mod server_protocol {
	// ServerProtocol.h
	pub(crate) const AS_PROTOCOL_VERSION: i32 = 1;
	
	// from the main enum there
	pub(crate) const AS_GET_DESKTOP: i32 = 0;
	pub(crate) const AS_REGISTER_INPUT_SERVER: i32 = 1;
	pub(crate) const AS_EVENT_STREAM_CLOSED: i32 = 2;

	// desktop definitions
	pub(crate) const AS_GET_WINDOW_LIST: i32 = 3;
	pub(crate) const AS_GET_WINDOW_INFO: i32 = 4;
	pub(crate) const AS_MINIMIZE_TEAM: i32 = 5;
	pub(crate) const AS_BRING_TEAM_TO_FRONT: i32 = 6;
	pub(crate) const AS_WINDOW_ACTION: i32 = 7;
	pub(crate) const AS_GET_APPLICATION_ORDER: i32 = 8;
	pub(crate) const AS_GET_WINDOW_ORDER: i32 = 9;
	
	// application definitions
	pub(crate) const AS_CREATE_APP: i32 = 10;
	pub(crate) const AS_DELETE_APP: i32 = 11;
	pub(crate) const AS_QUIT_APP: i32 = 12;
	pub(crate) const AS_ACTIVATE_APP: i32 = 13;
	pub(crate) const AS_APP_CRASHED: i32 = 14;
}

pub(crate) struct LinkSender {
	port: Port,
	cursor: Cursor<Vec<u8>>,
	current_message_start: u64
}

/// Class that sends the special server protocol to a port.
///
/// Flow
/// A message is started with start_message(). People can attach data using
/// the attach() function. A message is finished using the end_message().
/// In principal messages are queued until they are either sent explictly
/// using flush(), or when they hit a certain level (referred to as the
/// water mark).
///
/// Memory Management
/// There are two intermediate memory buffers: on is through a heap-allocated
/// data store with the MAX_BUFFER_SIZE (currently at 64kb). The other uses
/// the area system (not yet implemented).
impl LinkSender {
	pub(crate) fn start_message(&mut self, code: i32, mut size_hint: usize) -> Result<()> {		
		self.end_message(false)?;

		// Switch memory allocation method when size is larger than the buffersize
		size_hint += mem::size_of::<message_header>();
		if size_hint > MAX_BUFFER_SIZE {
			unimplemented!()
		}

		// Flush the message queue if we are going to hit the watermark
		if self.cursor.position() + size_hint as u64 > BUFFER_WATERMARK {
			self.flush(false)?
		}

		// Store the start of the header pos, and write the code to the buffer
		self.cursor.write(&(0 as i32).flatten());
		self.cursor.write(&code.flatten());
		self.cursor.write(&(0 as u32).flatten());

		Ok(())
	}
	
	pub(crate) fn cancel_message(&mut self) {
		self.cursor.set_position(self.current_message_start);
	}
	
	pub(crate) fn end_message(&mut self, needs_reply: bool) -> Result<()> {
		if self.current_message_start == self.cursor.position() {
			return Ok(());
		}
		let last_position = self.cursor.position();
		let size: i32 = (last_position - self.current_message_start) as i32;
		self.cursor.set_position(self.current_message_start);
		self.cursor.write(&size.flatten());
		if needs_reply {
			self.cursor.seek(SeekFrom::Current(mem::size_of::<u32>() as i64));
			self.cursor.write(&NEEDS_REPLY.flatten());
		}
		self.cursor.set_position(last_position);
		self.current_message_start = last_position;
		Ok(())
	}

	pub(crate) fn attach<T: Flattenable<T>>(&mut self, data: &T) -> Result<()> {
		// Check if we are currently in a message
		if self.cursor.position() == self.current_message_start {
			return Err(HaikuError::new(ErrorKind::InvalidInput, "Cannot attach data before starting a message"));
		}

		// Check if the data size will overrun the buffer, if so switch to area
		if data.flattened_size() > MAX_BUFFER_SIZE {
			unimplemented!();
		}

		// Write data to the buffer
		self.cursor.write(&data.flatten());

		Ok(())
	}

	pub(crate) fn attach_string(&mut self, data: &str) -> Result<()> {
		// A string consists of the size (as i32) followed by the string,
		// without the \nul terminator
		if data.len() > MAX_STRING_SIZE {
			return Err(HaikuError::new(ErrorKind::InvalidInput, "String too long"));
		}

		let size = data.len() as i32;
		self.cursor.write(&size.flatten());
		self.cursor.write(data.as_bytes());

		Ok(())
	}

	pub(crate) fn flush(&mut self, needs_reply: bool) -> Result<()> {
		self.end_message(needs_reply)?;
		if self.current_message_start == 0 {
			return Ok(());
		}

		let buffer = &self.cursor.get_ref().as_slice()[0..self.current_message_start as usize];
		
		self.port.write(LINK_CODE, buffer)?;

		self.cursor.set_position(0);
		self.current_message_start = 0;
		Ok(())
	}

	pub(crate) fn get_port_id(&self) -> port_id {
		self.port.get_port_id()
	}
}

pub(crate) struct LinkReceiver {
	port: Port
}

impl LinkReceiver {
	pub(crate) fn get_next_message(&mut self) -> Result<i32> {
		Ok((0))
	}
	
	pub(crate) fn read<T: Flattenable<T>>(&mut self, data: &T) -> Result<()> {
		Ok(())
	} 
}

pub(crate) struct ServerLink {
	pub(crate) sender: LinkSender,
	receiver: LinkReceiver
}

const APPSERVER_PORT_NAME: &str = "a<app_server";
const DEFAULT_PORT_CAPACITY: i32 = 100;

impl ServerLink {
	fn create_desktop_connection() -> Result<ServerLink> {
		let receiver_port = Port::create(APPSERVER_PORT_NAME, DEFAULT_PORT_CAPACITY)?; 

		let mut request = Message::new(server_protocol::AS_GET_DESKTOP as u32);
		let uid = unsafe { libc::getuid() };

		println!("uid: {}", uid);
		request.add_data("user", &(uid as i32));
		request.add_data("version", &server_protocol::AS_PROTOCOL_VERSION);
		match env::var_os("TARGET_SCREEN") {
			Some(target) => request.add_data("target", &String::from(target.to_str().unwrap())),
			None => ()
		}

		let server = Messenger::from_signature("application/x-vnd.Haiku-app_server", None)?;
		let reply = server.send_and_wait_for_reply(request)?;
		println!("{:?}", reply);

		let server_port: port_id = reply.find_data("port", 0)?;
		let sender_cursor = Cursor::new(Vec::with_capacity(INITIAL_BUFFER_SIZE));
		Ok(ServerLink {
			sender: LinkSender{ 
				port: Port::from_id(server_port).unwrap(), 
				cursor: sender_cursor,
				current_message_start: 0
			},
			receiver: LinkReceiver{ port: receiver_port } 
		})
	}
}

#[test]
fn test_server_link() {
	let mut link = ServerLink::create_desktop_connection().unwrap();
	// Create a mock looper port
	let looper_port = Port::create("mock_looper", 100).unwrap();
	// Simulate attaching a program
	link.sender.start_message(server_protocol::AS_CREATE_APP, 0).unwrap();
	link.sender.attach(&link.sender.get_port_id()).unwrap();
	link.sender.attach(&looper_port.get_port_id()).unwrap();
}

#[test]
fn test_link_sender_behaviour() {
	let sender_port = Port::create("mock_sender", DEFAULT_PORT_CAPACITY).unwrap();
	let sender_cursor = Cursor::new(Vec::with_capacity(INITIAL_BUFFER_SIZE));
	let mut sender = LinkSender {
		port: sender_port,
		cursor: sender_cursor,
		current_message_start: 0
	};
	// Scenario 1
	//  Start a message
	//  Attach an integer
	//  Attach a string
	//  Mark as reply needed
	//  Flush
	sender.start_message(99, 0).unwrap();
	sender.attach(&(-1 as i32)).unwrap();
	sender.attach_string("this is a test string").unwrap();
	assert_eq!(sender.cursor.position(), 41);
	sender.end_message(true).unwrap();
	let comparison: Vec<u8> = vec!(41, 0, 0, 0, 99, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 255, 21, 0, 0, 0, 116, 104, 105, 115, 32, 105, 115, 32, 97, 32, 116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103);
	assert_eq!(sender.cursor.get_ref(), &comparison);
	sender.flush(true).unwrap();
	assert_eq!(sender.cursor.position(), 0);

	// Scenario 2
	//  Start a message
	//  Flush as no-reply
	//  Check buffer (we do not discard the buffer)
	sender.start_message(100, 0).unwrap();
	sender.flush(false).unwrap();
	let comparison: Vec<u8> = vec!(12, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0);
	assert_eq!(&sender.cursor.get_ref()[0..12], comparison.as_slice());
	assert_eq!(sender.cursor.position(), 0);

	// Scenario 3
	// Start message 1 (no data)
	// Start message 2 with size hint above water mark
	// Check if first message is flushed by looking at the cursor position
	sender.start_message(101, 0).unwrap();
	sender.start_message(102, 2020).unwrap();
	assert_eq!(sender.cursor.position(), 12);
}

	
