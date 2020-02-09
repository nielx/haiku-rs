//
// Copyright 2019-2020, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::collections::{HashMap, VecDeque};
use std::env::args;
use std::mem;
use std::sync::{Arc, Mutex, atomic};

use haiku_sys::{thread_info, thread_id, find_thread, get_thread_info, port_id, team_id};

use ::app::{Handler, Message, Messenger};
use ::app::looper::{HandlerType, Looper, LooperDelegate, NEXT_HANDLER_TOKEN};
use ::app::roster::{ROSTER, ApplicationRegistrationStatus};
use ::app::serverlink::{ServerLink, server_protocol};
use ::app::sys::{B_ARGV_RECEIVED, B_READY_TO_RUN, B_QUIT_REQUESTED, QUIT, B_PREFERRED_TOKEN, get_app_path};
use ::kernel::INFINITE_TIMEOUT;
use ::kernel::ports::Port;
use ::storage::MimeType;
use ::storage::sys::entry_ref;
use ::support::Result;

const LOOPER_PORT_DEFAULT_CAPACITY: i32 = 200;

pub struct Application<A> where A: ApplicationHooks + Send + 'static {
	state: Arc<Mutex<A>>,
	inner_looper: Looper<A>,
	signature: MimeType,
	link: ServerLink
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
		
		match ROSTER.add_application(&String::from(signature), &entry, app_flags,
			team, thread, port.get_port_id(), true) {
				Ok(_) => (),
				Err(_) => panic!("Error registering with the registrar")
		};

		// Set up some defaults
		let state = Arc::new(Mutex::new(initial_state));
		let default_looper_state = Box::new(ApplicationLooperState{});
		let context = Context {
			handler_messenger: Messenger::from_port(&port).unwrap(),
			looper: LooperDelegate{ messenger: Messenger::from_port(&port).unwrap() },
			application: ApplicationDelegate{ messenger: Messenger::from_port(&port).unwrap() },
			application_state: state.clone()
		};
		let mut handlers = HashMap::new();
		let handler_token = NEXT_HANDLER_TOKEN.fetch_add(1, atomic::Ordering::Relaxed);
		handlers.insert(handler_token, HandlerType::LooperState);
		let mut inner_looper = Looper {
			name: String::from("application"),
			port: port,
			message_queue: VecDeque::new(),
			handlers: handlers,
			preferred_handler: handler_token,
			context: context,
			state: default_looper_state,
			terminating: false
		};

		// Add the ARGV_RECEIVED message to the queue
		let mut argv_message = Message::new(B_ARGV_RECEIVED);
		argv_message.header.target = B_PREFERRED_TOKEN;
		argv_message.add_data("_internal", &true);
		inner_looper.message_queue.push_back(argv_message);

		// Add the READY_TO_RUN message to the queue
		let mut ready_message = Message::new(B_READY_TO_RUN);
		ready_message.header.target = B_PREFERRED_TOKEN;
		inner_looper.message_queue.push_back(ready_message);

		// Connect to the app_server
		let mut link = ServerLink::create_desktop_connection().unwrap();
		// AS_CREATE_APP:
		// Data: 1) port_id - receiver port of the serverlink
		//       2) port_id - looper port for this BApplication
		//       3) team_id - the team id for this application
		//       4) i32 - the handler ID token of this app
		//       5) &str - signature of this app
		link.sender.start_message(server_protocol::AS_CREATE_APP, 0).unwrap();
		link.sender.attach(&link.receiver.port.get_port_id()).unwrap();
		link.sender.attach(&inner_looper.port.get_port_id()).unwrap();
		link.sender.attach(&team).unwrap();
		link.sender.attach(&handler_token).unwrap();
		link.sender.attach_string(signature).unwrap();
		link.sender.flush(true).unwrap();
		let message = link.receiver.get_next_message(INFINITE_TIMEOUT).unwrap();
		if message.0 != 0 {
			panic!("Cannot register the application at the app_server");
		}
		let server_port: port_id = link.receiver.read(0).unwrap();
		let _: i32 = link.receiver.read(0).unwrap(); // area id, ignore for now
		let _: i32 = link.receiver.read(0).unwrap(); // team id, ignore for now
		link.sender.set_port(Port::from_id(server_port).unwrap());

		Self {
			state: state,
			inner_looper: inner_looper,
			signature: mime_type,
			link: link
		}
	}

	pub fn create_looper(&mut self, name: &str, initial_state: Box<dyn Handler<A> + Send>) -> Looper<A>
	{
		let port = Port::create(name, LOOPER_PORT_DEFAULT_CAPACITY).unwrap();
		let mut handlers = HashMap::new();
		let token = NEXT_HANDLER_TOKEN.fetch_add(1, atomic::Ordering::Relaxed);
		handlers.insert(token, HandlerType::LooperState);
		let context = Context {
			handler_messenger: Messenger::from_port(&port).unwrap(),
			looper: LooperDelegate{ messenger: Messenger::from_port(&port).unwrap() },
			application: ApplicationDelegate{ messenger: self.inner_looper.get_messenger() },
			application_state: self.state.clone()
		};
		Looper {
			name: String::from(name),
			port: port,
			message_queue: VecDeque::new(),
			handlers: handlers,
			preferred_handler: token,
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

		// Unregister from the app_server
		self.link.sender.start_message(B_QUIT_REQUESTED as i32, 0).unwrap();
		self.link.sender.flush(false).unwrap();
	}
}

/// Interact with the application object
pub struct ApplicationDelegate {
	pub messenger: Messenger
}

impl ApplicationDelegate {
	/// Send a message to the application to quit
	///
	/// This message will inform the application's Looper that you want to end
	/// the message loop. In effect this means that the application will no
	/// longer process messages.
	///
	/// Note that this request does not clean up any of the existing Loopers.
	pub fn quit(&self) {
		let message = Message::new(QUIT);
		self.messenger.send(message, &self.messenger);
	}
}

/// Execution context for a Handler
///
/// All Handers execute in the context of a Looper. The Context object is
/// passed to your Handler in the message_received() method, so that you can
/// interact with other parts of the system.
///
/// The context contains three messengers, that may be used to send messages
/// to certain destinations, or to use as reply addresses while sending
/// messages. Additionally, the Context gives access to the current
/// application state.
pub struct Context<A> where A: Send {
	/// The messenger to the current Handler
	///
	/// This Messenger is most useful as a reply address for any messages you
	/// are sending out.
	pub handler_messenger: Messenger,
	/// Interact with the current Looper
	///
	/// This is the Looper that is the context for the current message
	///
	/// Note that in some cases, this will be the same message queue as the
	/// application delegate.
	pub looper: LooperDelegate,
	/// Interact with the current Application object
	///
	/// This gives access to the global application struct
	pub application: ApplicationDelegate,
	/// Access the global Application state
	///
	/// This state is shared among the whole application. It is locked behind
	/// a Mutex, to allow thread-safe access. Note that using the application
	/// state is 'dangerous', in the sense that it may lead to deadlocks when
	/// two interdependent threads are waiting for access to it.
	/// Imagine this scenario:
	///   1. Looper A gets a reference to the application state, locking it.
	///   2. Looper A sends a Message to Looper B and waits for a reply
	///   3. Looper B receives the message. While handling that message, it
	///      tries to get a reference to the application state. Since the
	///      application state is already locks, both threads will be waiting
	///      for each other.
	///
	/// There are two best practises:
	///   1. Do not use synchronous messaging, unless you know what you are
	///      doing.
	///   2. If you do need access to the application state, use the lock()
	///      method of the Mutex (instead of get_mut()), and drop the Guard
	///      as soon as you are done.
	pub application_state: Arc<Mutex<A>>,
}

pub trait ApplicationHooks {
	fn ready_to_run(&mut self, _application: &ApplicationDelegate) {
	}
	
	fn message_received(&mut self, _application: &ApplicationDelegate, _message: &Message) {
	}

	fn argv_received(&mut self, _application: &ApplicationDelegate, _argv: Vec<String>) {
	}
}

struct ApplicationLooperState {}

impl<A> Handler<A> for ApplicationLooperState 
	where A: ApplicationHooks + Send + 'static 
{
	fn message_received(&mut self, context: &Context<A>, message: &Message) {
		let mut application_state = context.application_state.lock().unwrap();
		// Dispatch specific messages to particular application hooks
		match message.what() {
			B_ARGV_RECEIVED => {
				let argv = parse_argv(message);
				if argv.len() > 0 {
					application_state.argv_received(&context.application, argv);
				}
			},
			B_READY_TO_RUN => application_state.ready_to_run(&context.application),
			_ => application_state.message_received(&context.application, message)
		}
	}
}

// Convert a B_ARGV_RECEIVED message into a Vector with strings
fn parse_argv(message: &Message) -> Vec<String> {
	let internal = message.find_data::<bool>("_internal", 0).unwrap_or(false);
	let mut argv: Vec<String> = Vec::new();
	if internal {
		// parse argv
		for arg in args() {
			argv.push(arg);
		}
	} else {
		for i in 0.. {
			let arg = match message.find_data::<String>("argv", i) {
				Ok(arg) => arg,
				Err(_) => break
			};
			argv.push(arg);
		}
	}
	argv
}
		

/// Get the current team id and thread id
// TODO: some caching
pub(crate) fn get_current_team_and_thread() -> (team_id, thread_id) {
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
	use app::{Message};
	use app::sys::QUIT;
	
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
					context.application.messenger.send_and_ask_reply(response, &context.looper.messenger);
				},
				_ => panic!("We are not supposed to receive messages other than ADD_TO_COUNTER"),
			}
		}
	}
	
	struct ApplicationState {
		total_count: u32
	}
	
	impl ApplicationHooks for ApplicationState {
		fn ready_to_run(&mut self, application: &ApplicationDelegate) {
			println!("ready_to_run()");
		}
		
		fn message_received(&mut self, application: &ApplicationDelegate, message: &Message) {
			match message.what() {
				INFORM_APP_ABOUT_COUNTER => {
					self.total_count += 1;
					let count = message.find_data::<u32>("count", 0).unwrap();
					if count == 2 {
						// Quit the looper when the count hits 2
						let messenger = message.get_return_address().unwrap();
						// TODO:  We should not be using QUIT here, this is an internal detail
						//        In general, it should be resolved how we do inter-looper
						//        management
						messenger.send_and_ask_reply(Message::new(QUIT), &messenger);
					}
					println!("total count: {}", self.total_count);
				},
				_ => println!("application: {}", message.what())
			}
			
			// Check if we are done now
			if self.total_count == 4 {
				application.quit();
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
