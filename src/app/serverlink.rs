//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

// This module contains the private messaging interface between Haiku applications
// and the app server.

use std::env;

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
const MAX_BUFFER_SIZE: usize = 65536;
const NEEDS_REPLY: u32 = 0x01;

pub(crate) struct LinkSender {
	port: Port
}

impl LinkSender {
	pub(crate) fn start_message(&mut self, code: i32, minSize: usize) -> Result<()> {
		Ok(())
	}
	
	pub(crate) fn cancel_message(&mut self) {
	}
	
	pub(crate) fn end_message(needs_reply: bool) -> Result<()> {
		Ok(())
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
	sender: LinkSender,
	receiver: LinkReceiver
}

const APPSERVER_PORT_NAME: &str = "a<app_server";
const DEFAULT_PORT_CAPACITY: i32 = 100;

impl ServerLink {
	fn create_desktop_connection(capacity: i32) -> Result<ServerLink> {
		let receiver_port = Port::create(APPSERVER_PORT_NAME, DEFAULT_PORT_CAPACITY)?; 

		// TODO: find a place to put AS_GET_DESKTOP constant
		let mut request = Message::new(0);
		let uid = unsafe { libc::getuid() };
		
		println!("uid: {}", uid);
		request.add_data("user", &(uid as i32));
		request.add_data("version", &(1 as i32)); //find a place to put AS_PROTOCOL_VERSION
		match env::var_os("TARGET_SCREEN") {
			Some(target) => request.add_data("target", &String::from(target.to_str().unwrap())),
			None => ()
		}
		
		let server = Messenger::from_signature("application/x-vnd.Haiku-app_server", None)?;
		let reply = server.send_and_wait_for_reply(request)?;
		println!("{:?}", reply);
		
		let server_port: port_id = reply.find_data("port", 0)?;
		Ok(ServerLink {
			sender: LinkSender{ port: Port::from_id(server_port).unwrap() },
			receiver: LinkReceiver{ port: receiver_port } 
		})
	}
}

#[test]
fn test_server_link() {
	let link = ServerLink::create_desktop_connection(DEFAULT_PORT_CAPACITY).unwrap();
}

