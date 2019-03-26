//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use ::app::{Handler, Message, Messenger};
use ::app::looper::Looper;
use ::kernel::ports::Port;
use ::support::Result;

const LOOPER_PORT_DEFAULT_CAPACITY: i32 = 200;

pub struct Application<A> where A: Send + 'static {
	state: Arc<Mutex<A>>,
	inner_looper: Looper<A>
}

impl<A> Application<A> where A: Send {
	pub fn new(initial_state: A) -> Self {
		// Set up some defaults
		let port = Port::create("application", LOOPER_PORT_DEFAULT_CAPACITY).unwrap();
		let state = Arc::new(Mutex::new(initial_state));
		let context = Context {
			application_messenger: Messenger::from_port(&port).unwrap(),
			application_state: state.clone()
		};
		let inner_looper = Looper {
			name: String::from("application"),
			port: port,
			message_queue: VecDeque::new(),
			handlers: Vec::new(),
			context: context
		};
		
		Self {
			state: state,
			inner_looper: inner_looper,
		}
	}

	pub fn create_looper(&mut self, name: &str, initial_handler: Box<dyn Handler<A> + Send>) -> Looper<A>
	{
		let context = Context {
			application_messenger: self.inner_looper.get_messenger(),
			application_state: self.state.clone()
		};
		Looper {
			name: String::from(name),
			port: Port::create(name, LOOPER_PORT_DEFAULT_CAPACITY).unwrap(),
			message_queue: VecDeque::new(),
			handlers: vec![initial_handler],
			context: context
		}
	}
	
	pub fn run(&mut self) -> Result<()> {
		println!("Running application looper!");
		self.inner_looper.looper_task();
		Ok(())
	}
}

pub struct Context<A> where A: Send {
	pub application_messenger: Messenger,
	pub application_state: Arc<Mutex<A>>
}

#[cfg(test)]
mod tests {
	use super::*;
	use app::{Message};
	
	struct CountLooperState {
		count: u32
	}
	
	impl Handler<ApplicationState> for CountLooperState {
		fn message_received(&mut self, context: &mut Context<ApplicationState>, message: &Message) {
			println!("{}", message.what());
		}
	}
	
	struct ApplicationState {
		total_count: u32
	}
	
	impl Handler<ApplicationState> for ApplicationState {
		fn message_received(&mut self, context: &mut Context<ApplicationState>, message: &Message) {
			println!("application: {}", message.what());
		}
	}
	
	#[test]
	fn looper_test() {
		let looper_state_1 = Box::new(CountLooperState{ count: 0 });
		let looper_state_2 = Box::new(CountLooperState{ count: 0 });
		let application_state = ApplicationState{ total_count: 0 };

		let mut application = Application::new(application_state);

		let looper_1 = application.create_looper("looper 1", looper_state_1);
		let messenger_1 = looper_1.get_messenger();
		let looper_2 = application.create_looper("looper 2", looper_state_2);
		let messenger_2 = looper_2.get_messenger();
		assert!(looper_1.run().is_ok());
		assert!(looper_2.run().is_ok());
		let mut message = Message::new(1234);
		messenger_1.send_and_ask_reply(message, &messenger_2);
		let mut message = Message::new(5678);
		messenger_1.send_and_ask_reply(message, &messenger_2);
	}
}
