//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use haiku_sys::{B_MESSAGE_TYPE, port_id};
use haiku_sys::errors::B_OK;

use ::app::message::Message;
use ::app::roster::{LAUNCH_ROSTER, ROSTER};
use ::app::sys::*;
use ::kernel::ports::Port;
use ::kernel::teams::Team;
use ::support::{ErrorKind, Flattenable, HaikuError, Result};

/// A messenger is a helper that sends Messages through ports
pub struct Messenger {
	port: Port,
}


impl Messenger {
	pub fn from_port(port: &Port) -> Option<Messenger> {
		return Messenger::from_port_id(port.get_port_id());
	}
	
	pub fn from_port_id(port: port_id) -> Option<Messenger> {
		let result = Port::from_id(port);
		match result {
			Some(borrowed_port) => Some(Messenger{ port: borrowed_port }),
			None => None
		}
	}

	pub fn from_signature(signature: &str, team: Option<&Team>) -> Result<Messenger> {
		// TODO: Haiku's C++ version of Messenger also stores the team, we may want that in ours too
		let mut app_argv_only: bool = false;
		let mut port: port_id = -1;
		if team.is_some() {
			match ROSTER.get_running_app_info(team.unwrap()) {
				Some(info) => {
					if info.signature != signature {
						return Err(HaikuError::new(ErrorKind::InvalidInput, "signature did not match the signature of the team"));
					}
					port = info.port;
					app_argv_only = info.is_argv_only();
				},
				None => return Err(HaikuError::new(ErrorKind::NotFound, "cannot find application info for this team"))
			}
		} else {
			// no team is given, first see if the launch roster has data
			match LAUNCH_ROSTER.get_data(signature) {
				Ok(data) => {
					if data.what() == B_OK as u32  {
						port = data.find_data("port", 0).unwrap_or(-1);
					}
				},
				Err(_) => ()
			}

			if port < 0 {
				match ROSTER.get_app_info(signature) {
					Some(info) => {
						port = info.port;
						app_argv_only = info.is_argv_only();
					},
					None => return Err(HaikuError::new(ErrorKind::NotFound, "Cannot find a running app with this signature")),
				}
			}
		}

		// check whether the app flags say B_ARGV_ONLY
		if app_argv_only {
			return Err(HaikuError::new(ErrorKind::NotAllowed, "This application only accepts command line arguments"));
		}
		Ok(Messenger{ port: Port::from_id(port).unwrap() })
	}

	pub fn send_and_wait_for_reply(&self, mut message: Message) -> Result<Message> {
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
		let result = p.read()?;
		Message::unflatten(&result.1.as_slice())
	}
	
	pub fn send_and_ask_reply(&self, mut message: Message, reply_to: &Messenger) -> Result<()> {
		let info = reply_to.port.get_info()?;
		// Fill out header info
		message.header.target = B_PREFERRED_TOKEN; //TODO: allow other options
		message.header.reply_port = reply_to.port.get_port_id();
		message.header.reply_target = B_NULL_TOKEN;
		message.header.reply_team = info.team.get_team_id();
		message.header.flags |= MESSAGE_FLAG_WAS_DELIVERED;

		message.header.flags |= MESSAGE_FLAG_REPLY_REQUIRED;
		message.header.flags &= !MESSAGE_FLAG_REPLY_DONE;

		let flattened_message = message.flatten();
		self.port.write(B_MESSAGE_TYPE as i32, &flattened_message).ok();
		Ok(())
	}
}

#[test]
fn test_messenger_creation() {
	// Find team by Port (use known port "system:launch_daemon")
	let port = Port::find("system:launch_daemon").unwrap();
	assert!(Messenger::from_port(&port).is_some());

	// Find team by signature with no known team_id
	assert!(Messenger::from_signature("application/x-vnd.Be-TRAK", None).is_ok());
	assert!(Messenger::from_signature("application/doesnotexist", None).is_err());

	// Find team by signature and team id
	let tracker_info = ROSTER.get_app_info("application/x-vnd.Be-TRAK").unwrap();
	assert!(Messenger::from_signature("application/x-vnd.Be-TRAK", Some(&Team::from(tracker_info.team).unwrap())).is_ok());
}

#[test]
fn test_synchronous_message_sending() {
	use libc::getuid;
	// B_GET_LAUNCH_DATA is defined as 'lnda' see LaunchDaemonDefs.h
	let constant: u32 = haiku_constant!('l', 'n', 'd', 'a');
	let mut app_data_message = Message::new(constant);
	app_data_message.add_data("name", &String::from("application/x-vnd.haiku-registrar"));
	let uid = unsafe { getuid() };
	app_data_message.add_data("user", &(uid as i32));
	let port = Port::find("system:launch_daemon").unwrap();
	let messenger = Messenger::from_port(&port).unwrap();
	let response_message = messenger.send_and_wait_for_reply(app_data_message).unwrap();
	assert!(response_message.is_reply());
	assert!(response_message.is_source_remote());
	println!("response_message: {:?}", response_message);
	let port = response_message.find_data::<i32>("port", 0).unwrap();
	println!("registrar port: {}", port);
}

