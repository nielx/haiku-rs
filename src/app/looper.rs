//
// Copyright 2019, 2020 Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::collections::{HashMap, VecDeque};
use std::marker::Send;
use std::sync::atomic;
use std::sync::atomic::AtomicI32;
use std::thread;
use std::time::Duration;

use ::app::{Context, Message, Messenger, B_QUIT_REQUESTED, QUIT};
use app::sys::B_PREFERRED_TOKEN;
use ::kernel::ports::Port;
use ::kernel::INFINITE_TIMEOUT;
use ::support::{ErrorKind, Flattenable, HaikuError, Result};

/// A trait for the ability to process messages in the context of a looper
///
/// Objects that implement this trait, can be added to the messaging queues
/// of loopers.
pub trait Handler<A> where A: Send + 'static {
	/// Handle a message
	///
	/// When a Looper receives a message, this method is called for you to
	/// handle it.
	/// TODO: Example
	fn message_received(&mut self, context: &Context<A>, message: &Message);
}

pub(crate) enum HandlerType<A> where A: Send + 'static {
	OwnedHandler(Box<dyn Handler<A> + Send>),
	LooperState
}

/// A system that receives and processes messages in a separate thread
///
/// Loopers are a core Haiku concept. Haiku embraces the multithreaded
/// application model, where the functionality of the application is split
/// up over different threads with a specific job. The best example of the use
/// of a looper is in that every Window is its own Looper.
///
/// Haiku's design consists of an Application with one or more Loopers. Each
/// Looper then functions as an independent message queue, which receives
/// messages from other parts of the application, or from external applications,
/// and then dispatches these to Handlers. In this implementation of the API,
/// any object may be a Handler, as long as it implements the Handler trait.
/// A Looper has at least one Handler, which is referred to as the InitialState.
///
/// To create a new Looper, you call the Application::create_looper() method.
/// This method takes as an argument a Box<YourState>, which is a required
/// instance of a type that that you define and use. The only requirement is
/// that this State implements the Handler trait, so that it may receive and
/// process messages. The State becomes the preferred Handler by default.
///
/// After creating a Looper, you can add additional Handlers to it, with
/// the `add_handler()` method. After you add the Handler, the Looper takes
/// ownership, this means that you can no longer manipulate the object
/// yourself. It is possible to mark a Handler as the preferred Handler by
/// using the add_preferred_handler() method. This will override any previously
/// selected preferred Handler.
///
/// Once the Looper and its Handlers are set up, you can start the message
/// queue by using the run() method. Calling this method will transfer
/// ownership of the Looper object to the new Looper thread. Any interaction
/// you may want with that thread, should be done through the messaging
/// system.
///
/// A Looper will continue to run until the it gets a request to quit. This
/// can be done by sending the B_QUIT_REQUESTED message. Additionally, a
/// Looper will quit when the Application is quitting.
pub struct Looper<A> where A: Send + 'static {
	pub(crate) name: String,
	pub(crate) port: Port,
	pub(crate) message_queue: VecDeque<Message>,
	pub(crate) handlers: HashMap<i32, HandlerType<A>>,
	pub(crate) preferred_handler: i32,
	pub(crate) context: Context<A>,
	pub(crate) state: Box<dyn Handler<A> + Send>,
	pub(crate) terminating: bool
}

impl<A> Looper<A> where A: Send + 'static {	
	/// Get the name for this Looper
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Get a Messenger for this looper
	///
	/// This Messenger by default points to the preferred Handler.
	pub fn get_messenger(&self) -> Messenger {
		Messenger::from_port(&self.port).unwrap()
	}

	/// Start the message loop
	///
	/// When you use this method, the Looper ownership of the Looper object
	/// will be transferred to the Looper's thread. The message processing
	/// will start, until the Looper is requested to quit.
	pub fn run(mut self) -> Result<()> {
		let child = thread::spawn(move || {
			println!("[{}] Running looper", self.name());
			self.looper_task();
		});
		Ok(())
	}

	/// Add a Handler to the message queue
	///
	/// The handler may be any object that implements the Handler trait. The
	/// object should be created on the heap (as a Box).
	pub fn add_handler(&mut self, handler: Box<dyn Handler<A> + Send>) {
		self.handlers.insert(NEXT_HANDLER_TOKEN.fetch_add(1, atomic::Ordering::Relaxed), HandlerType::OwnedHandler(handler));
	}

	/// Add a preferred Handler to the message queue
	///
	/// Like the add_handler() method, this method takes ownership of any
	/// Handler. In addition, this method will also set the Handler as the
	/// preferred Handler of this Looper. This will overwrite the previously
	/// set preferred Handler.
	pub fn add_preferred_handler(&mut self, handler: Box<dyn Handler<A> + Send>) {
		let token = NEXT_HANDLER_TOKEN.fetch_add(1, atomic::Ordering::Relaxed);
		self.handlers.insert(token, HandlerType::OwnedHandler(handler));
		self.preferred_handler = token;
	}

	pub(crate) fn looper_task(&mut self) {
		loop {
			println!("[{}] outer loop", self.name());

			// Try to read the first message from the port
			// This will block until there is a message
			// Note that we check for anything in the queue because the
			// Application object puts a READY_TO_RUN in the queue, and
			// we want to guarantee that that one is processed, without
			// getting stuck on waiting for messages in the port.
			if self.message_queue.len() == 0 {
				match self.read_message_from_port(INFINITE_TIMEOUT) {
					Ok(message) => self.message_queue.push_back(message),
					Err(e) => {
						println!("[{}] Error getting message: {:?}", self.name(), e); 
						continue;
					}
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
			while dispatch_next_message && ! self.terminating {
				let message = self.message_queue.pop_front();
				
				if message.is_none() {
					dispatch_next_message = false;
				} else {
					let message = message.unwrap();
					let mut handler_token = message.header.target;
					println!("[{}] Handling message {:?}", self.name(), message);
					if handler_token == B_PREFERRED_TOKEN {
						handler_token = self.preferred_handler;
					}

					let handler = match self.handlers.get_mut(&handler_token) {
						Some(handler) => handler,
						None => continue, //If we are not the addressee, continue next
					};

					match message.what() {
						B_QUIT_REQUESTED => {},
						QUIT => { self.terminating = true; },
						_ => {
							self.context.handler_messenger.set_token(handler_token);
							match handler {
								HandlerType::OwnedHandler(h) => {
									h.message_received(&self.context, &message);
								},
								HandlerType::LooperState => {
									self.state.message_received(&self.context, &message);
								}
							}	
						}
					}
				}

				if self.terminating {
					break;
				}

				match self.port.get_count() {
					Ok(count) => {
						if count > 0 {
							dispatch_next_message = false;
						}
					},
					Err(e) => println!("Error getting the port count: {:?}", e)
				}
			}
			if self.terminating {
				println!("[{}] terminating looper", self.name());
				break;
			}
			println!("[{}] at the end of the outer loop", self.name());
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

/// Interact with the associated looper
///
/// The looper controls the message flow. This delegate allows you to access
/// it's messenger and implements some convenience methods to interact.
pub struct LooperDelegate {
	/// The Messenger to the current Looper
	pub messenger: Messenger
}

impl LooperDelegate {
	/// Send a message to the looper to end the message loop
	///
	/// This message will inform the Looper that you want to end
	/// the message loop. In effect this means that the Looper will stop
	/// processing messages and will free any resources that are associated
	/// with it.
	pub fn quit(&self) {
		let message = Message::new(QUIT);
		self.messenger.send(message, &self.messenger);
	}
}

/// The following global counter creates new unique tokens to identify handlers.
// The original class also kept an accounting of the associated Handler objects
// so that they can be addressed directly. This does not fit the memory
// ownership model of Rust.
// Additionally, the original implementation recycles tokens once Handers
// disappear. Let's consider this a TODO.
// Also: the token counter is part of the looper module here, but it might
// as well be in the application object... to do.
pub(crate) static NEXT_HANDLER_TOKEN: AtomicI32 = AtomicI32::new(2);

