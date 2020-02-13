//
// Copyright 2019-2020, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

// This module contains the private messaging interface between Haiku applications
// and the app server.

use std::env;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::mem;
use std::str;
use std::time::Duration;

use libc::ssize_t;
use haiku_sys::{port_id, read_port_etc, port_buffer_size_etc, B_TIMEOUT};
use haiku_sys::errors::B_INTERRUPTED;

use ::app::message::Message;
use ::app::messenger::Messenger;
use ::kernel::ports::Port;
use ::support::{Result, Flattenable, HaikuError, ErrorKind};

const LINK_CODE: i32 = haiku_constant!('_','P','T','L') as i32;
const INITIAL_BUFFER_SIZE: usize = 2048;
const BUFFER_WATERMARK: u64 = INITIAL_BUFFER_SIZE as u64 - 24;
const MAX_BUFFER_SIZE: usize = 65536;
const MAX_STRING_SIZE: usize = 4096;
const NEEDS_REPLY: u32 = 0x01;
const HEADER_SIZE: usize = 12;

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
pub(crate) struct LinkSender {
	port: Port,
	cursor: Cursor<Vec<u8>>,
	current_message_start: u64
}

// TODO: Re-enable dead_code warnings when class is further tested
#[allow(dead_code)]
impl LinkSender {
	pub(crate) fn start_message(&mut self, code: i32, mut size_hint: usize) -> Result<()> {		
		self.end_message(false)?;

		// Switch memory allocation method when size is larger than the buffersize
		size_hint += HEADER_SIZE;
		if size_hint > MAX_BUFFER_SIZE {
			unimplemented!()
		}

		// Flush the message queue if we are going to hit the watermark
		if self.cursor.position() + size_hint as u64 > BUFFER_WATERMARK {
			self.flush(false)?
		}

		// Store the start of the header pos, and write the code to the buffer
		self.cursor.write(&(0 as i32).flatten()).unwrap();
		self.cursor.write(&code.flatten()).unwrap();
		self.cursor.write(&(0 as u32).flatten()).unwrap();

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
		self.cursor.write(&size.flatten()).unwrap();
		if needs_reply {
			self.cursor.seek(SeekFrom::Current(mem::size_of::<u32>() as i64)).unwrap();
			self.cursor.write(&NEEDS_REPLY.flatten()).unwrap();
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
		self.cursor.write(&data.flatten()).unwrap();

		Ok(())
	}

	pub(crate) fn attach_string(&mut self, data: &str) -> Result<()> {
		// A string consists of the size (as i32) followed by the string,
		// without the \nul terminator
		if data.len() > MAX_STRING_SIZE {
			return Err(HaikuError::new(ErrorKind::InvalidInput, "String too long"));
		}

		let size = data.len() as i32;
		self.cursor.write(&size.flatten()).unwrap();
		self.cursor.write(data.as_bytes()).unwrap();

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

	pub(crate) fn set_port(&mut self, port: Port) {
		self.port = port;
	}
}

// Struct that implements receiving the special link messages from the server protocol
//
// Messages are read from the internal buffer, if there are no more messages
// in the internal buffer, new data will be requested from the port.
//
// This class implements the iterator protocol to get new messages.
//
// The class contains an internal buffer with messages that use the link
// protocol format. The buffer contains 0 or more messages. The cursor points
// to the current reading position.
//
// A message consists of a message header, and 0 or more bytes of data. The
// methods on this object basically deserialize the data into formats that the
// consumer uses.
//
// The reader can be in 3 states:
//  1. the buffer is empty
//  2. the buffer contains data; the cursor is at the beginning of a message
//  3. the buffer contains data; the cursor is in the data stream of a message
//
// REVIEW: the LinkReceiver currently aborts on invalid data. It could be argued
//         that it should me made more fault-intolerant (either for allowing future
//         changes to the protocol, or because we are basically operating on foreign
//         data).
#[derive(Debug,PartialEq)]
enum Position {
	Start(usize),
	Inside(usize, usize),
	Empty
}

pub(crate) struct LinkReceiver {
	pub(crate) port: Port,
	buffer: Vec<u8>,
	position: Position
}

impl Iterator for LinkReceiver {
	type Item = (u32, usize, bool);

	fn next(&mut self) -> Option<Self::Item> {
		self.get_next_message(Duration::new(0, 0))
	}
}

// TODO: re-enable dead code warnings when class is further developed
#[allow(dead_code)]
impl LinkReceiver {
	pub(crate) fn get_next_message(&mut self, timeout: Duration) -> Option<(u32, usize, bool)> {
		// check if the current buffer is empty or if we are at the end
		let fetch: bool = match self.position {
			Position::Empty => true,
			Position::Start(_) => false,
			Position::Inside(_, end) => {
				if end == self.buffer.len() {
					true
				} else {
					false
				}
			}
		};
		if fetch {
			match self.fetch_from_port(timeout) {
				Ok(_) => (),
				Err(_) => return None
			}
		}
		self.get_next_message_from_buffer()
	}

	// read data. If T has a variable size, the size parameter needs to be passed. If T is a fixed size, it is ignored.
	pub(crate) fn read<T: Flattenable<T>>(&mut self, mut size: usize) -> Result<T> {
		let (pos, end) = match self.position {
			Position::Start(_) => return Err(HaikuError::new(ErrorKind::NotAllowed, "LinkReceiver currently is at the start of a message, read the header data first")),
			Position::Inside(pos,end) => (pos, end),
			Position::Empty => return Err(HaikuError::new(ErrorKind::NotAllowed, "LinkReceiver currently is not reading a message, read the header data first")),
		};

		// Do some checks on size
		if T::is_fixed_size() {
			size = mem::size_of::<T>();
		}
		if size > (end - pos) {
			return Err(HaikuError::new(ErrorKind::InvalidData, "size of the data is larger than the remainder of the buffer"));
		}
		// Try to unflatten the data
		let result = T::unflatten(&self.buffer[pos..pos+size]);
		if result.is_ok() {
			self.position = Position::Inside(pos + size, end);
		}
		result
	}

	// Helper function to read a string
	pub(crate) fn read_string(&mut self) -> Result<String> {
		let (mut pos, end) = match self.position {
			Position::Start(_) => return Err(HaikuError::new(ErrorKind::NotAllowed, "LinkReceiver currently is at the start of a message, read the header data first")),
			Position::Inside(pos,end) => (pos, end),
			Position::Empty => return Err(HaikuError::new(ErrorKind::NotAllowed, "LinkReceiver currently is not reading a message, read the header data first")),
		};

		let size = self.read::<i32>(0)?;
		// if the size < 0 we are probably reading invalid data, so rewind
		if size < 0 {
			self.position = Position::Inside(pos, end);
			return Err(HaikuError::new(ErrorKind::InvalidData, "Invalid size for string"))
		} else {
			pos += mem::size_of::<i32>();
		}

		if size == 0 {
			Ok(String::new())
		} else {
			let size: usize = size as usize;
			if size > (end - pos) {
				return Err(HaikuError::new(ErrorKind::InvalidData, "size of the data is larger than the remainder of the buffer"));
			}
			// don't use regular unflattening, as our strings are not \0 terminated
			let data = match str::from_utf8(&self.buffer[pos..pos+size]) {
				Ok(borrowed_data) => String::from(borrowed_data),
				Err(_) => return Err(HaikuError::new(ErrorKind::InvalidData, "the string contains invalid characters"))
			};
			self.position = Position::Inside(pos+size, end);
			Ok(data)
		}
	}

	/// Fetch new messages from port.
	/// If there are no new messages (and the port_buffer_size_etc() request times out), return Ok()
	fn fetch_from_port(&mut self, timeout: Duration) -> Result<()> {
		let timeout_ms = timeout.as_secs() as i64 * 1_000_000 + timeout.subsec_micros() as i64;
		// check if we need to adjust the size of the buffer
		let mut buffer_size: ssize_t =  B_INTERRUPTED as ssize_t;
		while buffer_size == (B_INTERRUPTED as ssize_t) {
			buffer_size = unsafe { port_buffer_size_etc(self.port.get_port_id(), B_TIMEOUT, timeout_ms) };
		}
		if buffer_size < 0 {
			return Err(HaikuError::from_raw_os_error(buffer_size as i32));
		} else if buffer_size == 0 {
			// no new data, reset the buffer nontheless.
			unsafe { self.buffer.set_len(0); };
			self.position = Position::Empty;
			return Ok(());
		}

		let buffer_size = buffer_size as usize; // convert to usize

		if buffer_size > MAX_BUFFER_SIZE {
			panic!("LinkReceiver buffer size is larger than the maximum buffer size");
		}

		if buffer_size > self.buffer.capacity() {
			let additional = buffer_size - self.buffer.len();
			self.buffer.reserve(additional as usize);
		}

		// read data from port
		let pbuffer = self.buffer.as_mut_ptr();
		let mut len: ssize_t = B_INTERRUPTED as ssize_t;
		let mut type_code: i32 = 0;
		while len == (B_INTERRUPTED as ssize_t) {
			len = unsafe { read_port_etc(self.port.get_port_id(), &mut type_code, pbuffer, buffer_size, B_TIMEOUT, 0) };
		}
		if len > 0 && len != buffer_size as isize {
			panic!("read_port does not return the expected number of bytes");
		}

		if len < 0 {
			self.invalidate_buffer();
			Err(HaikuError::from_raw_os_error(len as i32))
		} else if type_code != LINK_CODE {
			panic!("read_port does not return the expected type code");
		} else {
			unsafe { self.buffer.set_len(len as usize); };
			self.position = Position::Start(0);
			Ok(())
		}
	}

	fn get_next_message_from_buffer(&mut self) -> Option<(u32, usize, bool)> {
		match self.position {
			Position::Start(pos) => self.read_message_header(pos),
			Position::Inside(_, end) => self.read_message_header(end),
			Position::Empty => None
		}
	}

	fn read_message_header(&mut self, pos: usize) -> Option<(u32, usize, bool)> {
		// Check if the buffer is large enough to have a header
		if self.buffer.len() - pos < HEADER_SIZE {
			self.invalidate_buffer();
			return None;
		}

		// REVIEW: sizes are hardcoded here
		let size = i32::unflatten(&self.buffer[pos..pos+4]).unwrap() as usize;
		let code = u32::unflatten(&self.buffer[pos+4..pos+8]).unwrap();
		let flags = u32::unflatten(&self.buffer[pos+8..pos+12]).unwrap();

		if size < HEADER_SIZE || size > (self.buffer.len() - pos) {
			self.invalidate_buffer();
			return None;
		}

		// Move the position to after the header
		self.position = Position::Inside(pos + HEADER_SIZE, pos + size);

		Some((code, size, (flags & NEEDS_REPLY) != 0))
	}

	fn invalidate_buffer(&mut self) {
		self.buffer.clear();
		self.position = Position::Empty;
	}
}

pub(crate) struct ServerLink {
	pub(crate) sender: LinkSender,
	pub(crate) receiver: LinkReceiver
}

const APPSERVER_PORT_NAME: &str = "a<app_server";
const DEFAULT_PORT_CAPACITY: i32 = 100;

impl ServerLink {
	pub(crate) fn create_desktop_connection() -> Result<ServerLink> {
		let receiver_port = Port::create(APPSERVER_PORT_NAME, DEFAULT_PORT_CAPACITY)?; 

		let mut request = Message::new(server_protocol::AS_GET_DESKTOP as u32);
		let uid = unsafe { libc::getuid() };

		println!("uid: {}", uid);
		request.add_data("user", &(uid as i32)).unwrap();
		request.add_data("version", &server_protocol::AS_PROTOCOL_VERSION).unwrap();
		match env::var_os("TARGET_SCREEN") {
			Some(target) => request.add_data("target", &String::from(target.to_str().unwrap())).unwrap(),
			None => ()
		}

		let server = Messenger::from_signature("application/x-vnd.Haiku-app_server", None)?;
		let reply = server.send_and_wait_for_reply(request, None)?;
		println!("{:?}", reply);

		let server_port: port_id = reply.find_data("port", 0)?;
		let sender_cursor = Cursor::new(Vec::with_capacity(INITIAL_BUFFER_SIZE));
		Ok(ServerLink {
			sender: LinkSender{ 
				port: Port::from_id(server_port).unwrap(), 
				cursor: sender_cursor,
				current_message_start: 0
			},
			receiver: LinkReceiver{
				port: receiver_port,
				buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
				position: Position::Empty
			}
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
fn test_link_sender_receiver_behaviour() {
	let receiver_port = Port::create("mock_receiver", DEFAULT_PORT_CAPACITY).unwrap();
	let sender_port = Port::from_id(receiver_port.get_port_id()).unwrap();
	let sender_cursor = Cursor::new(Vec::with_capacity(INITIAL_BUFFER_SIZE));
	let mut sender = LinkSender {
		port: sender_port,
		cursor: sender_cursor,
		current_message_start: 0
	};
	let mut receiver = LinkReceiver {
		port: receiver_port,
		buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
		position: Position::Empty
	};

	// Scenario 1
	//  Start a message
	//    Attach an integer
	//    Attach a string
	//    Mark as reply needed
	//    Flush
	//  Receive the message
	//    Check the code and reply_needed
	//    Read the integer
	//    Read the string
	//  Check that there are no more messages in the queue
	sender.start_message(99, 0).unwrap();
	sender.attach(&(-1 as i32)).unwrap();
	let test_string = "this is a test string";
	sender.attach_string(test_string).unwrap();
	assert_eq!(sender.cursor.position(), 41);
	sender.end_message(true).unwrap();
	let comparison: Vec<u8> = vec!(41, 0, 0, 0, 99, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 255, 21, 0, 0, 0, 116, 104, 105, 115, 32, 105, 115, 32, 97, 32, 116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103);
	assert_eq!(sender.cursor.get_ref(), &comparison);
	sender.flush(true).unwrap();
	assert_eq!(sender.cursor.position(), 0);

	assert!(receiver.fetch_from_port(Duration::new(0,0)).is_ok());
	assert_eq!(&receiver.buffer, &comparison);
	assert_eq!(receiver.position, Position::Start(0));

	let (code, size, needs_reply) = receiver.get_next_message_from_buffer().unwrap();
	assert_eq!(size, 41);
	assert_eq!(code, 99);
	assert_eq!(needs_reply, true);
	assert_eq!(receiver.position, Position::Inside(12, 41));
	let data_1 = receiver.read::<i32>(165).unwrap(); // the size parameter should be ignored, since i32 is fixed size
	assert_eq!(data_1, -1);
	let data_2 = receiver.read_string().unwrap();
	assert_eq!(data_2, test_string);
	assert_eq!(receiver.position, Position::Inside(41, 41));
	assert!(receiver.get_next_message_from_buffer().is_none());
	assert_eq!(receiver.position, Position::Empty);

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
	//  Start message 1 (no data)
	//  Start message 2 with size hint above water mark
	//  Check if first message is flushed by looking at the cursor position
	sender.start_message(101, 0).unwrap();
	sender.start_message(102, 2020).unwrap();
	assert_eq!(sender.cursor.position(), 12);

	// Receiver check (scenario 2 + 3)
	let mut count: u32 = 100;
	for (code, _size, reply) in receiver {
		assert_eq!(count, code);
		assert_eq!(reply, false);
		count += 1;
		// at message 101 we are at the last message in queue, then add the 102
		// message to see if the code properly fetches new messages from port.
		if code == 101 {
			sender.flush(false).unwrap();
		}
	}
	assert_eq!(count, 103);
}
