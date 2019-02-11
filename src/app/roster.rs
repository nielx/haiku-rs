use libc::{c_char, dev_t, getuid, ino_t};
use haiku_sys::{B_MIME_TYPE_LENGTH, B_FILE_NAME_LENGTH, port_id, team_id, thread_id};
use std::{mem, ptr};
use std::str::{Utf8Error, from_utf8};

use ::app::message::Message;
use ::app::messenger::Messenger;
use ::kernel::helpers;
use ::kernel::ports::Port;
use ::kernel::teams::Team;
use ::support::flattenable::Flattenable;

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
		if response.is_none() {
			return None
		}
		let response_message = response.unwrap();
		let port = response_message.find_data::<i32>("port", 0).unwrap();
		let team = response_message.find_data::<i32>("team", 0).unwrap();
		Some((Port::from_id(port).unwrap(), Team::from(team).unwrap()))
	}
}


pub struct Roster {
	messenger: Messenger
}


impl Roster {
	pub fn get_app_list(&self) -> Option<Vec<Team>> {
		let request = Message::new(haiku_constant!('r','g','a','l'));
		let response = self.messenger.send_and_wait_for_reply(request);

		if response.is_none() {
			return None;
		}

		let response = response.unwrap();
		if response.what == haiku_constant!('r','g','s','u') {
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
	
	pub fn get_running_app_info(&self, team: &Team) -> Option<AppInfo> {
		let mut request = Message::new(haiku_constant!('r','g','a','i'));
		request.add_data("team", &team.get_team_id());
		let response = self.messenger.send_and_wait_for_reply(request);
		
		if response.is_none() {
			return None;
		}
		
		let response = response.unwrap();
		if response.what == haiku_constant!('r','g','s','u') {
			let flat_app_info = response.find_data::<FlatAppInfo>("app_info", 0).unwrap();
			return Some(flat_app_info.to_app_info());
		}
		return None;
	}
}


const B_REG_APP_INFO_TYPE: u32 = haiku_constant!('r','g','a','i');


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
	fn str_from_array_with_or_without_nul(buf: &[u8]) -> Result<&str, Utf8Error> {
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

	fn unflatten(buffer: &[u8]) -> Option<FlatAppInfo> {
		if mem::size_of::<FlatAppInfo>() != buffer.len() {
			return None;
		}
		let app_info: FlatAppInfo = unsafe { ptr::read(buffer.as_ptr() as *const _) };
		Some(app_info)
	}
}


pub struct AppInfo {
	pub thread: thread_id,
	pub team: team_id,
	pub port: port_id,
	pub flags: u32,
	pub path: String,
	pub signature: String,
}


lazy_static! {
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
