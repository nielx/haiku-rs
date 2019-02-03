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


lazy_static! {
	pub static ref ROSTER: Roster = {
		// Get a connection with the registrar
		let roster_data = LaunchRoster::init().get_data("application/x-vnd.haiku-registrar").expect("Cannot connect to the Registrar!");
		Roster{ messenger: Messenger::from_port_id(roster_data.0.get_port_id()).unwrap() }
	};
}
		
