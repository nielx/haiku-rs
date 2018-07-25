//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::mem::{size_of};
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
				hash_table: [255, 255, 255, 255, 255]
			}
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
		unimplemented!();
	}
}

impl Flattenable<Message> for Message {
	fn type_code() -> u32 {
		B_MESSAGE_TYPE
	}
	
	fn flattened_size(&self) -> usize {
		// TODO: support for fields
		return size_of::<message_header>();
		
		// From BMessage::FlattenedSize()
		// sizeof(message_header) + num_fields * sizeof(field_header) + data_len
	}
	
	fn is_fixed_size() -> bool {
		false
	}
	
	fn flatten(&self) -> Vec<u8> {
		// TODO: add headers and fields
		let bytes: &[u8] = unsafe { 
			from_raw_parts((&self.header as *const message_header) as *const u8, size_of::<message_header>())
		};
		bytes.to_vec()
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
			header: header_ref.clone()
		})
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
	let constant: u32 = ((('r' as u32) << 24) + (('g' as u32) << 16) + (('a' as u32) << 8) + ('l' as u32));
	let mut app_list_message = Message::new(constant);
	let port = Port::find("system:roster").unwrap();
	let mut response_message = app_list_message.send_and_wait_for_reply(&port);
}
	
