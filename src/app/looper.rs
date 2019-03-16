//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::collections::VecDeque;
use std::marker::Send;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ::app::{Message, Messenger};
use ::kernel::ports::Port;
use ::kernel::INFINITE_TIMEOUT;
use ::support::{ErrorKind, Flattenable, HaikuError, Result};

pub trait Handler {
	type ConcreteLooper;
	
	fn message_received(&mut self, looper: &ConcreteLooper, message: &Message);
}

pub struct Looper<A, T> where T: Handler + Send + 'static, A: Send + 'static {
	pub(crate) state: Box<T>,
	pub(crate) name: String,
	pub(crate) port: Port,
	pub(crate) message_queue: VecDeque<Message>,
	pub application_messenger: Messenger,
	pub application_state: Arc<Mutex<A>>
}

impl<A, T> Looper<A, T> where T: Handler + Send, A: Send + 'static {	
	pub fn name(&self) -> &str {
		&self.name
	}
	
	pub fn get_messenger(&self) -> Messenger {
		Messenger::from_port(&self.port).unwrap()
	}
	
	pub fn start_looper(mut self) -> Result<()> {
		let child = thread::spawn(move || {
			println!("Running looper {}", &self.name());
			self.looper_task();
		});
		Ok(())
	}
	
	pub(crate) fn looper_task(&mut self) {
		loop {
			println!("outer loop");

			// Try to read the first message from the port
			// This will block until there is a message
			match self.read_message_from_port(INFINITE_TIMEOUT) {
				Ok(message) => self.message_queue.push_back(message),
				Err(e) => {
					println!("Error getting message: {:?}", e); 
					continue;
				}
			}
			
			// Fetch next messages
			let message_count = self.port.get_count().unwrap();
			for _ in 0..message_count {
				// use timeout of 0 because we know there is a next message
				match self.read_message_from_port(Duration::new(0,0)) {
					Ok(message) => self.message_queue.push_back(message),
					Err(e) => {
						println!("Error getting message: {:?}", e); 
						break;
					}
				}
			}
			
			// Handle messages, until we have new messages waiting in the
			// queue, this is the inner loop
			let mut dispatch_next_message = true;
			while dispatch_next_message {
				let message = self.message_queue.pop_front();
				
				if message.is_none() {
					dispatch_next_message = false;
				} else {
					let message = message.unwrap();
					println!("Handling message {:?}", message);
					
					// Todo: support handler tokens and targeting
					
					self.state.message_received(&message);
				}
				
				// Todo: check if the looper should terminate
				
				match self.port.get_count() {
					Ok(count) => {
						if count > 0 {
							dispatch_next_message = false;
						}
					},
					Err(e) => println!("Error getting the port count: {:?}", e)
				}
			}
	
			println!("ending now");
			break;
		}
	}

	fn read_message_from_port(&self, timeout: Duration) -> Result<Message> {
		// TODO: handle B_INTERRUPTED?
		let (type_code, buffer) = self.port.try_read(timeout)?;
		if type_code as u32 == Message::type_code() {
			let message = Message::unflatten(&buffer)?;
			Ok(message)
		} else {
			Err(HaikuError::new(ErrorKind::InvalidData, "the data on the looper's port does not contain a Message"))
		}
	}
}
