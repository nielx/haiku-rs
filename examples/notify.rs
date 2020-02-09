//
// Copyright 2020, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

extern crate getopts;
extern crate haiku;

use getopts::Options;
use haiku::app::{Application, ApplicationDelegate, ApplicationHooks, Notification, NotificationType};

const SIGNATURE: &str = "application/x-vnd.HaikuRS-notify";

struct NotifyApp {
	options: Options,
	notification: Option<Notification>
}

impl ApplicationHooks for NotifyApp {
	fn ready_to_run(&mut self, application: &ApplicationDelegate) {
		match &self.notification {
			Some(n) => n.send(&application.messenger, None).expect("Error sending notification"),
			None => print_usage("notify", &self.options)
		}
		application.quit();
	}

	fn argv_received(&mut self, _application: &ApplicationDelegate, argv: Vec<String>) {
		// we need at least one argument
		if argv.len() <= 1 {
			return;
		}
		let mut matches = match self.options.parse(&argv[1..]) {
			Ok(m) => m,
			Err(f) => { 
				println!("{}", f.to_string());
				return;
			}
		};

		// Check if the help parameter was supplied; if so don't do anything
		if matches.opt_present("h") {
			return;
		}

		// Start building the notification based on the options
		let mut notification = Notification::default();

		// Make sure there is a message
		if matches.free.len() != 1 {
			println!("cannot find the required MESSAGE parameter");
			return;
		}
		notification.content = matches.free.pop();

		// verify the type
		if matches.opt_present("t") {
			let t = matches.opt_str("t").unwrap();
			notification.notification_type = match t.as_str() {
				"information" => NotificationType::Information,
				"important" => NotificationType::Important,
				"error" => NotificationType::Error,
				"progress" => NotificationType::Progress,
				_ => {
					println!("Invalid TYPE parameter. Please pass one of 'information', 'important', 'error' or 'progress'");
					return
				}
			};
		}

		// add a title
		if matches.opt_present("T") {
			notification.title = matches.opt_str("T");
		}

		// add a group
		if matches.opt_present("g") {
			notification.group = matches.opt_str("g");
		}

		// add an id
		if matches.opt_present("i") {
			notification.id = matches.opt_str("i");
		}

		// check for progress
		if matches.opt_present("p") {
			if notification.notification_type != NotificationType::Progress {
				println!("You can only set the progress parameter when the notification type is of `progress`");
				return;
			}
			let progress_string = matches.opt_str("p").unwrap();
			let progress = match progress_string.parse::<f32>() {
				Ok(p) => p,
				Err(_) => {
					println!("Invalid value for the progress parameter");
					return;
				}
			};

			if progress < 0.0 || progress > 1.0 {
				println!("Enter a value between 0.0 and 1.0 for the progress parameter");
				return;
			}
			notification.progress = progress;
		}
 
		// Store the notification for display on ready_to_run
		self.notification = Some(notification);
	}
}

fn build_options() -> Options {
	let mut options = Options::new();
	options.optopt("t", "type", "the type of option (information, important, error, progress)", "TYPE");
	options.optopt("T", "title", "title for your notification", "TITLE");
	options.optopt("g", "group", "the group name for this notification", "GROUP");
	options.optopt("i", "id", "the id that uniquely identifies this notification", "ID");
	options.optopt("p", "progress", "the value between 0.0 and 1.0 that expresses the progress, defaults to 0.0", "PROGRESS");
	options.optflag("h", "help", "print this help menu");
	options
}

fn print_usage(program: &str, opts: &Options) {
	let brief = format!("Usage: {} [options] MESSAGE", program);
	print!("{}", opts.usage(&brief));
}	

fn main() {
	let state = NotifyApp{ options: build_options(), notification: None };
	let app = Application::new(SIGNATURE, state);
	app.run().unwrap();
}
