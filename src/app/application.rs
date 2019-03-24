//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use ::app::{Handler, Messenger};
use ::app::looper::Looper;
use ::kernel::ports::Port;

const LOOPER_PORT_DEFAULT_CAPACITY: i32 = 200;

pub struct Application<A> where A: Handler<A> + Send + 'static {
	state: Arc<Mutex<A>>,
	port: Port
}

impl<A> Application<A> where A: Handler<A> + Send{
	pub fn new(initial_state: A) -> Self {
		Self {
			state: Arc::new(Mutex::new(initial_state)),
			port: Port::create("application", LOOPER_PORT_DEFAULT_CAPACITY).unwrap(),
		}
	}

	pub fn create_looper<L>(&mut self, name: &str, initial_state: Box<L>) -> Looper<A, L>
		where L: Handler<A> + Send + 'static
	{
		let context = Context {
			application_messenger: Messenger::from_port(&self.port).unwrap(),
			application_state: self.state.clone()
		};
		Looper {
			state: initial_state,
			name: String::from(name),
			port: Port::create(name, LOOPER_PORT_DEFAULT_CAPACITY).unwrap(),
			message_queue: VecDeque::new(),
			context: context
		}
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
		fn message_received(&mut self, context: &Context<ApplicationState>, message: &Message) {
			println!("{}", message.what());
		}
	}
	
	struct ApplicationState {
		total_count: u32
	}
	
	impl Handler<ApplicationState> for ApplicationState {
		fn message_received(&mut self, context: &Context<ApplicationState>, message: &Message) {
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
