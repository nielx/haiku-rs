use libc::{c_char, dev_t, getuid, ino_t};
use haiku_sys::{B_MIME_TYPE_LENGTH, B_FILE_NAME_LENGTH, port_id, team_id, thread_id, status_t};
use haiku_sys::errors::{B_ERROR, B_OK};
use std::{mem, ptr};
use std::result;
use std::str::{Utf8Error, from_utf8};

use ::app::message::Message;
use ::app::messenger::Messenger;
use ::kernel::helpers;
use ::kernel::ports::Port;
use ::kernel::teams::Team;
use ::support::{ErrorKind, Flattenable, HaikuError, Result};
use ::storage::sys::entry_ref;

pub(crate) struct LaunchRoster {
	messenger: Messenger
}

impl LaunchRoster {
	fn init() -> LaunchRoster {
		let port = Port::find("system:launch_daemon").expect("Cannot find the launch daemon");
		let roster_messenger = Messenger::from_port(&port).expect("Cannot connect to the launch daemon");
		LaunchRoster { messenger: roster_messenger }
	}

	/// Method to get the data that the launch_daemon has on an application
	///
	/// The result will be an Err() if there was something wrong with the
	/// communication. Otherwise, the response message that came in will be
	/// returned. Note that inside this message there still may be an error,
	/// but this is stored in the message.what.
	pub(crate) fn get_data(&self, signature: &str) -> Result<Message> {
		let constant: u32 = haiku_constant!('l','n','d','a');
		let mut message = Message::new(constant);
		// TODO: add support for &str as Flattenable
		message.add_data("name", &String::from(signature));
		let uid = unsafe { getuid() };
		message.add_data("user", &(uid as i32));

		// Send message
		let response = self.messenger.send_and_wait_for_reply(message, None)?;
		Ok(response)
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
		let response = self.messenger.send_and_wait_for_reply(request, None);

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
		let response = self.messenger.send_and_wait_for_reply(request, None);

		if response.is_err() {
			println!("Response.is err");
			return None;
		}

		let response = response.unwrap();
		if response.what() == haiku_constant!('r','g','s','u') {
			let flat_app_info = response.find_data::<FlatAppInfo>("app_info", 0).unwrap();
			return Some(flat_app_info.to_app_info());
		}
		return None;
	}

	/// Get the information of an application with a certain signature
	///
	/// If there is a problem connecting tot the registrar, this method
	/// will return None.
	/// Get the information of a running application
	///
	/// If there is a problem connecting to the registrar, this method
	/// will return None.
	pub fn get_app_info(&self, signature: &str) -> Option<AppInfo> {
		let mut request = Message::new(haiku_constant!('r','g','a','i'));
		request.add_data("signature", &String::from(signature));
		let response = self.messenger.send_and_wait_for_reply(request, None);
		
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
		let response = self.messenger.send_and_wait_for_reply(request, None)?;
		if response.what() == B_REG_SUCCESS {
			if !full_registration && team < 0 {
				let token: i32 = match response.find_data("token", 0) {
					Ok(token) => token,
					Err(_) => return Err(HaikuError::new(ErrorKind::InvalidData, "No token for preregistration by Registrar"))
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
				Err(HaikuError::new(ErrorKind::InvalidData, "Invalid registration response by Registrar"))
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

		let response = self.messenger.send_and_wait_for_reply(request, None)?;
		if response.what() == B_REG_SUCCESS {
			let registered: bool = response.find_data("registered", 0).unwrap_or(false);
			let pre_registered: bool = response.find_data("pre-registered", 0).unwrap_or(false);
			let app_info: Option<AppInfo> = match response.find_data::<FlatAppInfo>("app_info", 0) {
				Ok(info) => Some(info.to_app_info()),
				Err(_) => None
			};
			if (pre_registered || registered) && app_info.is_none() {
				Err(HaikuError::new(ErrorKind::InvalidData, "The Registrar returned an invalid response"))
			} else if pre_registered {
				Ok(ApplicationRegistrationStatus::PreRegistered(app_info.unwrap()))
			} else if registered {
				Ok(ApplicationRegistrationStatus::Registered(app_info.unwrap()))
			} else {
				Ok(ApplicationRegistrationStatus::NotRegistered)
			}
		} else {
			let errno: i32 = response.find_data("error", 0).unwrap_or(-1);
			Err(HaikuError::new(ErrorKind::InvalidData, format!("The Registrar returned an error on request: {}", errno)))
		}
	}

	/// Unregister a previously registered application
	pub(crate) fn remove_application(&self, team: team_id) -> Result<()> {
		// B_REG_REMOVE_APP
		let mut request = Message::new(haiku_constant!('r','g','r','a'));
		request.add_data("team", &team);

		let response = self.messenger.send_and_wait_for_reply(request, None)?;
		if response.what() == B_REG_SUCCESS {
			Ok(())
		} else {
			let error: status_t = response.find_data("error", 0).unwrap_or(B_ERROR);
			Err(HaikuError::from_raw_os_error(error))
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

	fn unflatten(buffer: &[u8]) -> Result<FlatAppInfo> {
		if mem::size_of::<FlatAppInfo>() != buffer.len() {
			return Err(HaikuError::new(ErrorKind::InvalidData, "the buffer is smaller than the flattened app info struct"));
		}
		let app_info: FlatAppInfo = unsafe { ptr::read(buffer.as_ptr() as *const _) };
		Ok(app_info)
	}
}


// Supporting constants for AppInfo
const B_SINGLE_LAUNCH: u32 = 0x0;
const B_MULTIPLE_LAUNCH: u32 = 0x1;
const B_EXCLUSIVE_LAUNCH: u32 = 0x2;
// B_LAUNCH_MASK 0x3
const B_BACKGROUND_APP: u32 = 0x4;
const B_ARGV_ONLY: u32 = 0x8;
// B_APP_INFO_RESERVED1_ 0x10000000


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
	/// You probably want to use the `launch_type()` function to get them
	pub flags: u32,
	/// The path of the executable
	pub path: String,
	/// The mime type that represents the application's signature
	pub signature: String,
}

///  Launch Types
///
/// Haiku Applications have three launch types: single launch, multiple launch
/// and exclusive launch. This can be a property on the executable file, and it
/// is also stored in Haiku's Registrar. This property is part of the `AppInfo`
pub enum LaunchType {
	SingleLaunch,
	MultipleLaunch,
	ExclusiveLaunch
}

impl AppInfo {
	/// Get the LaunchType for this application
	pub fn launch_type(&self) -> LaunchType {
		if self.flags & B_MULTIPLE_LAUNCH != 0 {
			LaunchType::MultipleLaunch
		} else if self.flags & B_EXCLUSIVE_LAUNCH != 0 {
			LaunchType::ExclusiveLaunch
		} else {
			LaunchType::SingleLaunch
		}
	}
	
	/// Determine if the application is a background application
	pub fn is_background(&self) -> bool {
		self.flags & B_BACKGROUND_APP != 0
	}
	
	/// Determine if the application only allows command line arguments for
	/// passing data, not messages.
	pub fn is_argv_only(&self) -> bool {
		self.flags & B_ARGV_ONLY != 0
	}
}


lazy_static! {
	pub(crate) static ref LAUNCH_ROSTER: LaunchRoster = {
		LaunchRoster::init()
	};
}


lazy_static! {
	/// The `ROSTER` gives access to a global `Roster` object that can be used
	/// to communicate with Haiku's registrar that tracks all the running Haiku
	/// applications.
	pub static ref ROSTER: Roster = {
		// Get a connection with the registrar
		let roster_data = LAUNCH_ROSTER.get_data("application/x-vnd.haiku-registrar").expect("Cannot connect to the launch_daemon to get info about the registrar!");
		if roster_data.what() != (B_OK as u32) {
			panic!("Cannot connect to the registrar");
		}
		let port: port_id = roster_data.find_data("port", 0).expect("Cannot find port info for registrar");
		Roster{ messenger: Messenger::from_port_id(port).unwrap() }
	};
}


#[test]
fn test_roster_get_app_list() {
	let app_list = ROSTER.get_app_list().unwrap();
	assert!(app_list.len() != 0);
}
