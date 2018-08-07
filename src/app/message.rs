//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::fmt;
use std::mem::{size_of, transmute_copy};
use std::ptr;
use std::slice::from_raw_parts;

use haiku_sys::B_MESSAGE_TYPE;
use haiku_sys::message::*;

use ::kernel::ports::Port;
use ::support::flattenable::Flattenable;

/// A rustean representation of a BMessage
///
/// Like in the Haiku C++ API, the message is identified by the `what` value.
/// This one can be accessed directly.
pub struct Message {
	/// A 32 bit integer that gives a signature to the message
	pub what: u32,
	header: message_header,
	fields: Vec<field_header>,
	data: Vec<u8>
}

impl Message {
	/// Create a new message with the signature `what`
	pub fn new(what: u32) -> Self {
		Message {
			what: what,
			header: message_header{
				message_format: MESSAGE_FORMAT_HAIKU,
				flags: MESSAGE_FLAG_VALID,
				what: what,
				current_specifier: -1,
				message_area: -1,
				target: B_NULL_TOKEN,
				reply_target: B_NULL_TOKEN,
				reply_port: -1,
				reply_team: -1,
				data_size: 0,
				field_count: 0,
				hash_table_size: 5,
				hash_table: [-1, -1, -1, -1, -1]
			},
			fields: Vec::new(),
			data: Vec::new()
		}
	}
	
	pub fn send_and_wait_for_reply(&mut self, target_port: &Port) -> Option<Message> {
		// Create a reply port (and maybe cache)
		let p: Port = Port::create("tmp_reply_port", 1).unwrap();
		let info = p.get_info().unwrap();
		
		// Fill out header info
		self.header.target = B_NULL_TOKEN; //TODO: allow other options
		self.header.reply_port = p.get_port_id();
		self.header.reply_target = B_NULL_TOKEN;
		self.header.reply_team = info.team.get_team_id();
		self.header.flags |= MESSAGE_FLAG_WAS_DELIVERED;
		
		self.header.flags |= MESSAGE_FLAG_REPLY_REQUIRED;
		self.header.flags &= !MESSAGE_FLAG_REPLY_DONE;
		
		let flattened_message = self.flatten();
		target_port.write(B_MESSAGE_TYPE as i32, &flattened_message);
		let result = p.read();
		match &result {
			Ok(data) => println!("message: {:?}", data),
			Err(error) => return None
		}
		let message = Message::unflatten(&result.unwrap().1.as_slice());
		match message {
			Some(msg) => println!("msg: {:?}", msg),
			None => println!("cannot convert")
		}
		None
	}
	
	pub fn add_field<T: Flattenable<T>>(&mut self, name: &str, data: &T) {
		// BMessage has an optimization where some headers are pre-allocated
		// to avoid reallocating the header array. We should implement this,
		// TODO: Vec::with_capacity can help with implementing this
		
		let mut flags: u16 = FIELD_FLAG_VALID;
		if T::is_fixed_size() {
			flags |= FIELD_FLAG_FIXED_SIZE;
		}
		
		let hash: u32 = self.hash_name(name) % self.header.hash_table_size;
		let mut current_index: i32 = self.header.hash_table[hash as usize];
		if current_index >= 0 {
			{
				let mut next_field: &field_header = &self.fields[current_index as usize];
				while next_field.next_field >= 0 {
					current_index = next_field.next_field;
					next_field = &self.fields[current_index as usize];
				}
			}
			self.fields.get_mut(current_index as usize).unwrap().next_field = current_index;
		} else {
			self.header.hash_table[hash as usize] = self.header.field_count as i32;
		}
		
		self.fields.push(field_header {
			flags: flags,
			name_length: name.len() as u16 + 1,
			field_type: T::type_code(),
			count: 1, // TODO: add more flexibility for multiple fields
			data_size: data.flattened_size() as u32,
			offset: self.header.data_size,
			next_field: -1
		});
		
		self.header.field_count += 1;
		
		// Store name to the vector
		let data_size = name.len() + 1 + data.flattened_size();
		self.data.reserve(data_size);
		for byte in name.as_bytes() {
			self.data.push(*byte);
		}
		self.data.push('\0' as u8);
				
		// Copy the data
		// TODO: we really want to add a flatten_into function to stop the
		// double copy
		let mut data = data.flatten();
		self.data.append(&mut data);
		
		// Update the header
		self.header.data_size += data_size as u32;
	}		

	fn hash_name(&self, name: &str) -> u32 {
		let mut result: u32 = 0;
		for byte in name.bytes() {
			result = (result << 7) ^ (result >> 24);
			result ^= byte as u32;
		}
		
		result ^= result << 12;
		result
	}
}

impl Flattenable<Message> for Message {
	fn type_code() -> u32 {
		B_MESSAGE_TYPE
	}
	
	fn flattened_size(&self) -> usize {
		return size_of::<message_header>() + size_of::<field_header>() * self.fields.len() + self.data.len();
	}
	
	fn is_fixed_size() -> bool {
		false
	}
	
	fn flatten(&self) -> Vec<u8> {
		let mut vec: Vec<u8> = vec![0;self.flattened_size()];
		// Copy message header
		{
			let (message_header_slice, _) = vec.as_mut_slice().split_at_mut(size_of::<message_header>());
			let message_header_bytes: &[u8] = unsafe { 
				from_raw_parts((&self.header as *const message_header) as *const u8, size_of::<message_header>())
			};
			message_header_slice.clone_from_slice(message_header_bytes);
		}
		// Copy field headers and data
		if self.fields.len() > 0 {
			{
				let (_, field_header_slice) = vec.as_mut_slice().split_at_mut(size_of::<message_header>());
				let field_header_bytes: &[u8] = unsafe { 
					from_raw_parts((self.fields.as_slice() as *const [field_header]) as *const u8, size_of::<field_header>())
				};
				unsafe {
					ptr::copy_nonoverlapping(field_header_bytes.as_ptr(), field_header_slice.as_mut_ptr(), size_of::<field_header>() * self.fields.len());
				}
			}
			{
				// Copy data
				let(_, data_slice) = vec.as_mut_slice().split_at_mut(size_of::<message_header>() + size_of::<field_header>() * self.fields.len());
				unsafe {
					ptr::copy_nonoverlapping(self.data.as_ptr(), data_slice.as_mut_ptr(), self.data.len());
				}
			}
		}
		
		vec
	}
	
	fn unflatten(buffer: &[u8]) -> Option<Message> {
		// minimum size is at least the header
		if buffer.len() < size_of::<message_header>() {
			return None;
		}
		// check the first 4 bytes and compare the message constant
		if buffer[0] != 'H' as u8 || buffer[1] != 'M' as u8 || buffer[2] != 'F' as u8 || buffer[3] != '1' as u8 {
			return None;
		}
		
		if buffer.len() > size_of::<message_header>() {
			// TODO: unflatten larger messages
			unimplemented!()
		}

		let data_ptr: *const u8 = buffer.as_ptr();
		let header_ptr: *const message_header = data_ptr as *const _;
		let header_ref: &message_header = unsafe { &*header_ptr };
		Some(Message{
			what: header_ref.what,
			header: header_ref.clone(),
			fields: Vec::new(), //TODO!
			data: Vec::new() //TODO!
		})
	}
}

impl fmt::Debug for Message {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// TODO: make this mirror BMessage::PrintToStream()
		let chars = unsafe { transmute_copy::<u32, [u8; 4]>(&self.what) };
		let mut print_chars = true;
		for ch in chars.iter() {
			if !(*ch as char).is_ascii_graphic() {
				print_chars = false;
				break;
			}
		}
		
		if print_chars {
			write!(f, "BMessage: ({:?})", (chars[3] as char, chars[2] as char, chars[1] as char, chars[0] as char))
		} else {
			write!(f, "BMessage: ({})", self.what)
		}
	}
}

#[test]
fn test_basic_message() {
	let msg_constant = 1234567890;
	let msg = Message::new(msg_constant);
	let flattened_msg = msg.flatten();
	let unflattened_msg = Message::unflatten(flattened_msg.as_slice()).unwrap();
	assert_eq!(unflattened_msg.what, msg_constant);
}

#[test]
fn test_synchronous_message_sending() {
	use kernel::ports::Port;
	use libc::getuid;
	// B_GET_LAUNCH_DATA is defined as 'lnda' see LaunchDaemonDefs.h
	let constant: u32 = ((('l' as u32) << 24) + (('n' as u32) << 16) + (('d' as u32) << 8) + ('a' as u32));
	let mut app_data_message = Message::new(constant);
	app_data_message.add_field("name", &String::from("application/x-vnd.haiku-registrar"));
	let uid = unsafe { getuid() };
	app_data_message.add_field("user", &(uid as i32));
	let port = Port::find("system:launch_daemon").unwrap();
	println!("Outgoing message: {:?}", app_data_message);
	let mut response_message = app_data_message.send_and_wait_for_reply(&port);
	
}

#[test]
fn test_message_flattening() {
	let constant: u32 = ((('a' as u32) << 24) + (('b' as u32) << 16) + (('c' as u32) << 8) + ('d' as u32));
	let basic_message = Message::new(constant);
	let flattened_message = basic_message.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 100, 99, 98, 97, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255);
	assert_eq!(flattened_message, comparison);

	// Second message
	let constant: u32 = ((('e' as u32) << 24) + (('f' as u32) << 16) + (('g' as u32) << 8) + ('h' as u32));
	let mut message_with_data = Message::new(constant);
	message_with_data.add_field("UInt8", &('a' as u8));
	message_with_data.add_field("UInt16", &(1234 as u16));
	let flattened_message = message_with_data.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 104, 103, 102, 101, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 16, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 3, 0, 6, 0, 84, 89, 66, 85, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 3, 0, 7, 0, 84, 72, 83, 85, 1, 0, 0, 0, 2, 0, 0, 0, 7, 0, 0, 0, 255, 255, 255, 255, 85, 73, 110, 116, 56, 0, 97, 85, 73, 110, 116, 49, 54, 0, 210, 4);
	assert_eq!(flattened_message, comparison);
}

	
