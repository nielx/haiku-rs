//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::time::Duration;

use haiku_sys::message::*;
use haiku_sys::{B_MESSAGE_TYPE, port_id};

use ::app::message::Message;
use ::kernel::ports::Port;
use ::support::flattenable::Flattenable;

/// A messenger is a helper that sends Messages through ports
pub struct Messenger {
	port: Port,
}


impl Messenger {
	pub fn from_port(port: &Port) -> Option<Messenger> {
		return Messenger::from_port_id(port.get_port_id());
	}
	
	pub fn from_port_id(port: port_id) -> Option<Messenger> {
		let mut result = Port::from_id(port);
		match result {
			Some(borrowed_port) => Some(Messenger{ port: borrowed_port }),
			None => None
		}
	}
	
	pub fn send_and_wait_for_reply(&self, mut message: Message) -> Option<Message> {
		// Create a reply port (TODO: maybe cache?)
		let p: Port = Port::create("tmp_reply_port", 1).unwrap();
		let info = p.get_info().unwrap();
		
		// Fill out header info
		message.header.target = B_PREFERRED_TOKEN; //TODO: allow other options
		message.header.reply_port = p.get_port_id();
		message.header.reply_target = B_NULL_TOKEN;
		message.header.reply_team = info.team.get_team_id();
		message.header.flags |= MESSAGE_FLAG_WAS_DELIVERED;
		
		message.header.flags |= MESSAGE_FLAG_REPLY_REQUIRED;
		message.header.flags &= !MESSAGE_FLAG_REPLY_DONE;

		let flattened_message = message.flatten();
		self.port.write(B_MESSAGE_TYPE as i32, &flattened_message).ok();
		let result = p.read();
		if result.is_err() {
			return None;
		}
		Message::unflatten(&result.unwrap().1.as_slice())
	}
}

#[test]
fn test_synchronous_message_sending() {
	use libc::getuid;
	// B_GET_LAUNCH_DATA is defined as 'lnda' see LaunchDaemonDefs.h
	let constant: u32 = ((('l' as u32) << 24) + (('n' as u32) << 16) + (('d' as u32) << 8) + ('a' as u32));
	let mut app_data_message = Message::new(constant);
	app_data_message.add_data("name", &String::from("application/x-vnd.haiku-registrar"));
	let uid = unsafe { getuid() };
	app_data_message.add_data("user", &(uid as i32));
	let port = Port::find("system:launch_daemon").unwrap();
	let mut messenger = Messenger::from_port(&port).unwrap();
	let mut response_message = messenger.send_and_wait_for_reply(app_data_message).unwrap();
	println!("response_message: {:?}", response_message);
	let port = response_message.find_data::<i32>("port", 0).unwrap();
	println!("registrar port: {}", port);
}

