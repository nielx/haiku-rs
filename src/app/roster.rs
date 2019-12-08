use libc::{c_char, dev_t, getuid, ino_t};
use haiku_sys::{B_MIME_TYPE_LENGTH, B_FILE_NAME_LENGTH, port_id, team_id, thread_id};
use std::{mem, ptr};
use std::result;
use std::str::{Utf8Error, from_utf8};

use ::app::message::Message;
use ::app::messenger::Messenger;
use ::kernel::helpers;
use ::kernel::ports::Port;
use ::kernel::teams::Team;
use ::support;
use ::support::{ErrorKind, Flattenable, HaikuError, Result};
use ::storage::sys::entry_ref;

struct LaunchRoster {
	messenger: Messenger
}

impl LaunchRoster {
	fn init() -> LaunchRoster {
		let port = Port::find("system:launch_daemon").expect("Cannot find the launch daemon");
		let roster_messenger = Messenger::from_port(&port).expect("Cannot connect to the launch daemon");
		LaunchRoster { messenger: roster_messenger }
	}
	
	fn get_data(&mut self, signature: &str) -> Option<(Port, Team)> {
		let constant: u32 = haiku_constant!('l','n','d','a');
		let mut message = Message::new(constant);
		// TODO: add support for &str as Flattenable
		message.add_data("name", &String::from(signature));
		let uid = unsafe { getuid() };
		message.add_data("user", &(uid as i32));

		// Send message
		let response = self.messenger.send_and_wait_for_reply(message);
		if response.is_err() {
			return None
		}
		let response_message = response.unwrap();
		let port = response_message.find_data::<i32>("port", 0).unwrap();
		let team = response_message.find_data::<i32>("team", 0).unwrap();
		Some((Port::from_id(port).unwrap(), Team::from(team).unwrap()))
	}
}

pub(crate) enum ApplicationRegistrationStatus {
	Registered(AppInfo),
	PreRegistered(AppInfo),
	NotRegistered
}

pub(crate) enum ApplicationRegistrationResult {
	Registered,
	PreRegistered(i32),
	OtherInstance(team_id, i32)
}


/// This struct provides information about applications on the Haiku system
///
/// This struct should be accessed through the static `ROSTER` reference. It
/// is automatically initialized to retrieve information from Haiku's
/// registrar.
pub struct Roster {
	messenger: Messenger
}

impl Roster {
	/// Get a list of teams that are currently running
	///
	/// If there is a problem connecting to the registrar, this method
	/// will return None.
	pub fn get_app_list(&self) -> Option<Vec<Team>> {
		let request = Message::new(haiku_constant!('r','g','a','l'));
		let response = self.messenger.send_and_wait_for_reply(request);

		if response.is_err() {
			return None;
		}

		let response = response.unwrap();
		if response.what() == haiku_constant!('r','g','s','u') {
			let count = match response.get_info("teams") {
				Some(info) => info.1,
				None => return None
			};
			let mut result: Vec<Team> = Vec::with_capacity(count);
			for index in 0..count {
				let team = response.find_data::<i32>("teams", index).unwrap();
				result.push(Team::from(team).unwrap());
			}
			return Some(result);
		}
		return None;
	}

	/// Get the information of a running application
	///
	/// If there is a problem connecting to the registrar, this method
	/// will return None.
	pub fn get_running_app_info(&self, team: &Team) -> Option<AppInfo> {
		let mut request = Message::new(haiku_constant!('r','g','a','i'));
		request.add_data("team", &team.get_team_id());
		let response = self.messenger.send_and_wait_for_reply(request);
		
		if response.is_err() {
			return None;
		}

		let response = response.unwrap();
		if response.what() == haiku_constant!('r','g','s','u') {
			let flat_app_info = response.find_data::<FlatAppInfo>("app_info", 0).unwrap();
			return Some(flat_app_info.to_app_info());
		}
		return None;
	}

	/// Register or preregister an app in the Registrar
	pub(crate) fn add_application(&self, signature: &String, entry: &entry_ref,
		flags: u32, team: team_id, thread: thread_id, port: port_id,
		full_registration: bool) -> Result<ApplicationRegistrationResult>
	{
		// B_REG_ADD_APP
		let mut request = Message::new(haiku_constant!('r','g','a','a'));
		request.add_data("signature", signature);
		request.add_data("ref", entry);
		request.add_data("flags", &flags);
		request.add_data("team", &team);
		request.add_data("thread", &thread);
		request.add_data("port", &port);
		request.add_data("full_registration", &full_registration);
		let response = self.messenger.send_and_wait_for_reply(request)?;
		if response.what() == B_REG_SUCCESS {
			if !full_registration && team < 0 {
				let token: i32 = match response.find_data("token", 0) {
					Ok(token) => token,
					Err(_) => return Err(support::HaikuError::new(support::ErrorKind::InvalidData, "No token for preregistration by Registrar"))
				};
				Ok(ApplicationRegistrationResult::PreRegistered(token))
			} else {
				Ok(ApplicationRegistrationResult::Registered)
			}
		} else {
			let token: Result<i32> = response.find_data("token", 0);
			let team: Result<team_id> = response.find_data("team", 0);
			if token.is_ok() && team.is_ok() {
				Ok(ApplicationRegistrationResult::OtherInstance(team.unwrap(), token.unwrap()))
			} else {
				Err(support::HaikuError::new(support::ErrorKind::InvalidData, "Invalid registration response by Registrar"))
			}
		}
	}

	/// Check on the registrar if the app is registered
	pub(crate) fn is_application_registered(&self, entry: &entry_ref,
		team: team_id, token: u32) -> Result<ApplicationRegistrationStatus>
	{
		// B_REG_IS_APP_REGISTERED
		let mut request = Message::new(haiku_constant!('r','g','i','p'));
		request.add_data("ref", entry);
		request.add_data("team", &team);
		request.add_data("token", &(token as i32));

		let response = self.messenger.send_and_wait_for_reply(request)?;
		if response.what() == B_REG_SUCCESS {
			let registered: bool = response.find_data("registered", 0).unwrap_or(false);
			let pre_registered: bool = response.find_data("pre-registered", 0).unwrap_or(false);
			let app_info: Option<AppInfo> = match response.find_data::<FlatAppInfo>("app_info", 0) {
				Ok(info) => Some(info.to_app_info()),
				Err(_) => None
			};
			if (pre_registered || registered) && app_info.is_none() {
				Err(support::HaikuError::new(support::ErrorKind::InvalidData, "The Registrar returned an invalid response"))
			} else if pre_registered {
				Ok(ApplicationRegistrationStatus::PreRegistered(app_info.unwrap()))
			} else if registered {
				Ok(ApplicationRegistrationStatus::Registered(app_info.unwrap()))
			} else {
				Ok(ApplicationRegistrationStatus::NotRegistered)
			}
		} else {
			let errno: i32 = response.find_data("error", 0).unwrap_or(-1);
			Err(support::HaikuError::new(support::ErrorKind::InvalidData, format!("The Registrar returned an error on request: {}", errno)))
		}
	}
}


const B_REG_APP_INFO_TYPE: u32 = haiku_constant!('r','g','a','i');
const B_REG_SUCCESS: u32 = haiku_constant!('r','g','s','u');


// It is not possible to safely get references from packed structs. Therefore
// we have a private FlatAppInfo to read data from messages, and a public
// AppInfo that can be used by consumers whichever way they want. See #46043
// on the rust-lang project
#[repr(packed)]
struct FlatAppInfo {
	pub thread: thread_id,
	pub team: team_id,
	pub port: port_id,
	pub flags: u32,
	pub ref_device: dev_t,
	pub ref_directory: ino_t,
	signature: [u8; B_MIME_TYPE_LENGTH],
	ref_name: [c_char; B_FILE_NAME_LENGTH + 1]
}


impl FlatAppInfo {
	fn to_app_info(&self) -> AppInfo {
		let signature = match FlatAppInfo::str_from_array_with_or_without_nul(&self.signature) {
			Ok(value) => String::from(value),
			Err(_) => String::new()
		};
		let path = match helpers::get_path_for_entry_ref(self.ref_device, self.ref_directory, self.ref_name.as_ptr()) {
			Ok(value) => String::from(value),
			Err(_) => String::new()
		};
		AppInfo {
			thread: self.thread,
			team: self.team,
			port: self.port,
			flags: self.flags,
			path: path,
			signature: signature 
		}
	}
	
	//Graciously borrowed from stackoverflow
	fn str_from_array_with_or_without_nul(buf: &[u8]) -> result::Result<&str, Utf8Error> {
		let len = buf.iter()
			.enumerate()
			.find(|&(_, &byte)| byte == 0)
			.map_or_else(|| buf.len(), |(len, _)| len);
		from_utf8(&buf[..len])
	}
}

impl Flattenable<FlatAppInfo> for FlatAppInfo {
	fn type_code() -> u32 {
		B_REG_APP_INFO_TYPE
	}

	fn is_fixed_size() -> bool {
		true
	}

	fn flattened_size(&self) -> usize {
		mem::size_of::<FlatAppInfo>()
	}

	fn flatten(&self) -> Vec<u8> {
		unimplemented!();
	}

	fn unflatten(buffer: &[u8]) -> support::Result<FlatAppInfo> {
		if mem::size_of::<FlatAppInfo>() != buffer.len() {
			return Err(support::HaikuError::new(support::ErrorKind::InvalidData, "the buffer is smaller than the flattened app info struct"));
		}
		let app_info: FlatAppInfo = unsafe { ptr::read(buffer.as_ptr() as *const _) };
		Ok(app_info)
	}
}


/// Contains the information about a running application
///
/// The information is provided by Haiku's registrar, and can be queried using
/// the methods of the `ROSTER` object
pub struct AppInfo {
	/// The thread id for the main thread or -1 if the application is not
	/// running
	pub thread: thread_id,
	/// The team id for the running instance or -1 if the application is not
	/// running
	pub team: team_id,
	/// The port that is listening to Messages to be processed by the
	/// application's main looper
	pub port: port_id,
	/// Any flags for this running application
	///
	/// Note that the flags are not yet implemented in this rust crate
	pub flags: u32,
	/// The path of the executable
	pub path: String,
	/// The mime type that represents the application's signature
	pub signature: String,
}


lazy_static! {
	/// The `ROSTER` gives access to a global `Roster` object that can be used
	/// to communicate with Haiku's registrar that tracks all the running Haiku
	/// applications.
	pub static ref ROSTER: Roster = {
		// Get a connection with the registrar
		let roster_data = LaunchRoster::init().get_data("application/x-vnd.haiku-registrar").expect("Cannot connect to the Registrar!");
		Roster{ messenger: Messenger::from_port_id(roster_data.0.get_port_id()).unwrap() }
	};
}


#[test]
fn test_roster_get_app_list() {
	let app_list = ROSTER.get_app_list().unwrap();
	assert!(app_list.len() != 0);
}
