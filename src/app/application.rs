//
// Copyright 2019-2020, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::collections::{HashMap, VecDeque};
use std::mem;
use std::sync::{Arc, Mutex, atomic};

use haiku_sys::{thread_info, thread_id, find_thread, get_thread_info, team_id};

use ::app::{B_READY_TO_RUN, Handler, Message, Messenger};
use ::app::looper::{HandlerType, Looper, NEXT_HANDLER_TOKEN};
use ::app::roster::{ROSTER, ApplicationRegistrationStatus};
use ::app::sys::get_app_path;
use ::kernel::ports::Port;
use ::storage::MimeType;
use ::storage::sys::entry_ref;
use ::support::Result;

const LOOPER_PORT_DEFAULT_CAPACITY: i32 = 200;

pub struct Application<A> where A: ApplicationHooks + Send + 'static {
	state: Arc<Mutex<A>>,
	inner_looper: Looper<A>,
	signature: MimeType
}

impl<A> Application<A> where A: ApplicationHooks + Send + 'static {
	pub fn new(signature: &str, initial_state: A) -> Self {
		// Check the signature
		let mime_type =  match MimeType::new(signature) {
			Some(t) => t,
			None => panic!("Invalid MimeType")
		};
		
		if mime_type.is_supertype_only() || (mime_type.get_supertype() != MimeType::new("application").unwrap()) {
			panic!("Invalid MimeType");
		}

		// Get an entry_ref for this path
		let path = get_app_path(0).expect("Cannot get the path for this executable");
		let entry = entry_ref::from_path(&path).expect("Cannot get the entry_ref for this executable");
		println!("path: {:?} entry: {:?}\n", path.to_str(), entry.name);

		// To do: see if the application file has any attributes set
		let app_flags: u32 = 1; //B_MULTIPLE_LAUNCH as B_REG_DEFAULT_APP_FLAGS

		// Register at the app server
		let port = Port::create("application", LOOPER_PORT_DEFAULT_CAPACITY).unwrap();
		let (team, thread) = get_current_team_and_thread();
		let registration = match ROSTER.is_application_registered(&entry, team, 0) {
			Ok(r) => r,
			Err(_) => panic!("Error communicating with the registrar about the registration status")
		};
		match registration {
			ApplicationRegistrationStatus::Registered(_) => (), //Ignored by the C++ implementation as well
			ApplicationRegistrationStatus::PreRegistered(_) => panic!("Pre registered applications are not implemented"),
			ApplicationRegistrationStatus::NotRegistered => (), //Ignored, now register
		};
		
		let registration = match ROSTER.add_application(&String::from(signature), &entry, app_flags,
			team, thread, port.get_port_id(), true) {
				Ok(r) => r,
				Err(_) => panic!("Error registering with the registrar")
		};

		// Set up some defaults
		let state = Arc::new(Mutex::new(initial_state));
		let default_looper_state = Box::new(ApplicationLooperState{});
		let context = Context {
			looper_messenger: Messenger::from_port(&port).unwrap(),
			application_messenger: Messenger::from_port(&port).unwrap(),
			application_state: state.clone()
		};
		let mut handlers = HashMap::new();
		handlers.insert(NEXT_HANDLER_TOKEN.fetch_add(1, atomic::Ordering::Relaxed), HandlerType::LooperState);
		let mut inner_looper = Looper {
			name: String::from("application"),
			port: port,
			message_queue: VecDeque::new(),
			handlers: handlers,
			context: context,
			state: default_looper_state,
			terminating: false
		};
		
		// Add the READY_TO_RUN message to the queue
		inner_looper.message_queue.push_back(Message::new(B_READY_TO_RUN));
		
		Self {
			state: state,
			inner_looper: inner_looper,
			signature: mime_type
		}
	}

	pub fn create_looper(&mut self, name: &str, initial_state: Box<dyn Handler<A> + Send>) -> Looper<A>
	{
		let port = Port::create(name, LOOPER_PORT_DEFAULT_CAPACITY).unwrap();
		let context = Context {
			looper_messenger: Messenger::from_port(&port).unwrap(),
			application_messenger: self.inner_looper.get_messenger(),
			application_state: self.state.clone()
		};
		let mut handlers = HashMap::new();
		handlers.insert(NEXT_HANDLER_TOKEN.fetch_add(1, atomic::Ordering::Relaxed), HandlerType::LooperState);
		Looper {
			name: String::from(name),
			port: port,
			message_queue: VecDeque::new(),
			handlers: handlers,
			context: context,
			state: initial_state,
			terminating: false
		}
	}
	
	pub fn run(&mut self) -> Result<()> {
		println!("Running application looper!");
		self.inner_looper.looper_task();
		Ok(())
	}
	
	pub fn get_messenger(&self) -> Messenger {
		self.inner_looper.get_messenger()
	}
}

impl<A> Drop for Application<A> where A: ApplicationHooks + Send + 'static {
	fn drop(&mut self) {
		// Unregister from Registrar
		let (team, _) = get_current_team_and_thread();
		let _ = ROSTER.remove_application(team);
	}
}

pub struct Context<A> where A: Send {
	pub looper_messenger: Messenger,
	pub application_messenger: Messenger,
	pub application_state: Arc<Mutex<A>>
}

pub trait ApplicationHooks {
	// TODO: the second argument for each hook now is a &Messenger. This should
	//       be a more context-like object in the future, that exposes more
	//       information about the application.
	fn quit_requested(&mut self, _application_messenger: &Messenger) -> bool {
		true
	}
	
	fn ready_to_run(&mut self, _application_messenger: &Messenger) {
	}
	
	fn message_received(&mut self, application_messenger: &Messenger, message: &Message);
}

struct ApplicationLooperState {}

impl<A> Handler<A> for ApplicationLooperState 
	where A: ApplicationHooks + Send + 'static 
{
	fn message_received(&mut self, context: &Context<A>, message: &Message) {
		let mut application_state = context.application_state.lock().unwrap();
		// Dispatch specific messages to particular application hooks
		match message.what() {
			B_READY_TO_RUN => application_state.ready_to_run(&context.application_messenger),
			_ => application_state.message_received(&context.application_messenger, message)
		}
	}
}

/// Get the current team id and thread id
// TODO: some caching
fn get_current_team_and_thread() -> (team_id, thread_id) {
	let mut info: thread_info = unsafe { mem::zeroed() };
	let (team, thread) = unsafe {
		if get_thread_info(find_thread(0 as *const i8), &mut info) == 0 {
			(info.team, info.thread)
		} else {
			(-1, -1)
		}
	};
	(team, thread)
}


#[cfg(test)]
mod tests {
	use super::*;
	use app::{Message, QUIT};
	
	const ADD_TO_COUNTER: u32 = haiku_constant!('C','O','+','+');
	const INFORM_APP_ABOUT_COUNTER: u32 = haiku_constant!('I','A','A','C');
	
	struct CountLooperState {
		count: u32
	}
	
	impl Handler<ApplicationState> for CountLooperState {
		fn message_received(&mut self, context: &Context<ApplicationState>, message: &Message) {
			match message.what() {
				ADD_TO_COUNTER => {
					self.count += 1;
					let mut response = Message::new(INFORM_APP_ABOUT_COUNTER);
					response.add_data("count", &self.count);
					context.application_messenger.send_and_ask_reply(response, &context.looper_messenger);
				},
				_ => panic!("We are not supposed to receive messages other than ADD_TO_COUNTER"),
			}
		}
	}
	
	struct ApplicationState {
		total_count: u32
	}
	
	impl ApplicationHooks for ApplicationState {
		fn ready_to_run(&mut self, _app_messenger: &Messenger) {
			println!("ready_to_run()");
		}
		
		fn message_received(&mut self, app_messenger: &Messenger, message: &Message) {
			match message.what() {
				INFORM_APP_ABOUT_COUNTER => {
					self.total_count += 1;
					let count = message.find_data::<u32>("count", 0).unwrap();
					if count == 2 {
						// Quit the looper when the count hits 2
						let messenger = message.get_return_address().unwrap();
						messenger.send_and_ask_reply(Message::new(QUIT), &messenger);
					}
					println!("total count: {}", self.total_count);
				},
				_ => println!("application: {}", message.what())
			}
			
			// Check if we are done now
			if self.total_count == 4 {
				app_messenger.send_and_ask_reply(Message::new(QUIT), &app_messenger);
			}
		}
	}
	
	#[test]
	fn looper_test() {
		let looper_state_1 = Box::new(CountLooperState{ count: 0 });
		let looper_state_2 = Box::new(CountLooperState{ count: 0 });
		let application_state = ApplicationState{ total_count: 0 };

		let mut application = Application::new("application/looper_test", application_state);

		let looper_1 = application.create_looper("looper 1", looper_state_1);
		let messenger_1 = looper_1.get_messenger();
		let looper_2 = application.create_looper("looper 2", looper_state_2);
		let messenger_2 = looper_2.get_messenger();
		assert!(looper_1.run().is_ok());
		assert!(looper_2.run().is_ok());
		
		// Create four count messages, two for each counter
		let app_messenger = application.get_messenger();
		let mut message = Message::new(ADD_TO_COUNTER);
		messenger_1.send_and_ask_reply(message, &app_messenger);
		let mut message = Message::new(ADD_TO_COUNTER);
		messenger_2.send_and_ask_reply(message, &app_messenger);
		let mut message = Message::new(ADD_TO_COUNTER);
		messenger_1.send_and_ask_reply(message, &app_messenger);
		let mut message = Message::new(ADD_TO_COUNTER);
		messenger_2.send_and_ask_reply(message, &app_messenger);

		application.run(); 
	}
}
