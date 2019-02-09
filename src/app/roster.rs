use libc::getuid;

use ::app::message::Message;
use ::app::messenger::Messenger;
use ::kernel::ports::Port;
use ::kernel::teams::Team;

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
