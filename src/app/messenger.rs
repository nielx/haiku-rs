//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::time::Duration;

use haiku_sys::{B_MESSAGE_TYPE, port_id};
use haiku_sys::errors::B_OK;

use ::app::message::Message;
use ::app::roster::{LAUNCH_ROSTER, ROSTER};
use ::app::sys::*;
use ::kernel::ports::Port;
use ::kernel::teams::Team;
use ::support::{ErrorKind, Flattenable, HaikuError, Result};

/// A messenger is a helper that sends Messages through ports
///
/// The best way to understand a messenger is that it is a communication pipe
/// to a specific Looper/Handler pair. This pipe can work within the
/// application, but it may also point to an external application, or a
/// system service.
pub struct Messenger {
	port: Port,
	token: i32
}


impl Messenger {
	/// Create a new Messenger from a port
	///
	/// This constructor will build a messenger that can send messages directly
	/// to a Port. The messages will not be targeted at any specific Handler,
	/// but instead will be targeted at the receivers' preferred Handler.
	///
	/// Note that this method does not do any validation on the port, including
	/// any checks whether it is a valid port, or whether the receiver on the
	/// other end actually expects any messages.
	pub fn from_port(port: &Port) -> Option<Messenger> {
		// Should this be a private method?
		return Some(Messenger{port: port.clone(), token: B_PREFERRED_TOKEN});
	}

	pub(crate) fn from_port_id(port: port_id) -> Option<Messenger> {
		let result = Port::from_id(port);
		match result {
			Some(borrowed_port) => Some(Messenger{ 
										port: borrowed_port,
										token: B_PREFERRED_TOKEN }),
			None => None
		}
	}

	/// Create a new Messenger for an external application.
	///
	/// This constructor will build a messenger that can send messages to an
	/// external application. To that end, it will try to find a running
	/// application with a `signature`. If there is a running application,
	/// you will receive a valid messenger.
	///
	/// Optionally, you may supply a team. This may be useful in case you know
	/// or expect that the application may be launched more than once, and you
	/// want to point to a specific team.
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
		Ok(Messenger{ port: Port::from_id(port).unwrap(), token: B_PREFERRED_TOKEN })
	}

	/// Synchronously send a Message and wait for a reply
	///
	/// Optionally you can add a timeout, with a maximum wait time. If you do
	/// not supply a timeout, this method will wait indefinitely.
	pub fn send_and_wait_for_reply(&self, mut message: Message, timeout: Option<Duration>) -> Result<Message> {
		// Create a reply port (TODO: maybe cache?)
		let p: Port = Port::create("tmp_reply_port", 1).unwrap();
		let info = p.get_info().unwrap();
		
		// Fill out header info
		message.header.target = self.token;
		message.header.reply_port = p.get_port_id();
		message.header.reply_target = B_NULL_TOKEN;
		message.header.reply_team = info.team.get_team_id();
		message.header.flags |= MESSAGE_FLAG_WAS_DELIVERED;
		
		message.header.flags |= MESSAGE_FLAG_REPLY_REQUIRED;
		message.header.flags &= !MESSAGE_FLAG_REPLY_DONE;

		let flattened_message = message.flatten();
		self.port.write(B_MESSAGE_TYPE as i32, &flattened_message).ok();
		
		let result = match timeout {
			Some(timeout) => p.try_read(timeout)?,
			None => p.read()?
		};
		Message::unflatten(&result.1.as_slice())
	}

	/// Aynchronously send a Message and ask for a reply
	///
	/// The Message will ask for a reply to the `reply_to` messenger.
	/// See the `send` method if you intend to send a message without asking
	/// for a reply.
	pub fn send_and_ask_reply(&self, mut message: Message, reply_to: &Messenger) -> Result<()> {
		let info = reply_to.port.get_info()?;
		// Fill out header info
		message.header.target = self.token;
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

	/// Aynchronously send a Message without asking a reply
	///
	/// See the `send_and_ask_reply` method if you intend to send a message
	/// that does ask for a reply. The `sender` argument is used to identify
	/// the sender.
	pub fn send(&self, mut message: Message, sender: &Messenger) -> Result<()> {
		let info = sender.port.get_info()?;
		// Fill out header info
		message.header.target = self.token;
		message.header.reply_port = sender.port.get_port_id();
		message.header.reply_target = sender.token;
		message.header.reply_team = info.team.get_team_id();
		message.header.flags |= MESSAGE_FLAG_WAS_DELIVERED;
		message.header.flags &= !MESSAGE_FLAG_REPLY_DONE;

		let flattened_message = message.flatten();
		self.port.write(B_MESSAGE_TYPE as i32, &flattened_message).ok();
		Ok(())
	}

	pub(crate) fn set_token(&mut self, token: i32) {
		self.token = token;
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
	app_data_message.add_data("name", &String::from("application/x-vnd.haiku-registrar")).unwrap();
	let uid = unsafe { getuid() };
	app_data_message.add_data("user", &(uid as i32)).unwrap();
	let port = Port::find("system:launch_daemon").unwrap();
	let messenger = Messenger::from_port(&port).unwrap();
	let response_message = messenger.send_and_wait_for_reply(app_data_message, None).unwrap();
	assert!(response_message.is_reply());
	assert!(response_message.is_source_remote());
	println!("response_message: {:?}", response_message);
	let port = response_message.find_data::<i32>("port", 0).unwrap();
	println!("registrar port: {}", port);
}

