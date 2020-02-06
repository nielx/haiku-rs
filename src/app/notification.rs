//
// Copyright 2020, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::path::Path;
use std::time::Duration;

use ::app::application::get_current_team_and_thread;
use ::app::{Message, Messenger, ROSTER};
use ::kernel::teams::Team;
use ::support::Result;

const NOTIFICATION_MESSAGE: u32 = haiku_constant!('n','s','s','m');
const NOTIFICATION_SERVER_SIGNATURE: &str = "application/x-vnd.Haiku-notification_server";

#[derive(PartialEq)]
pub enum NotificationType {
	Information,
	Important,
	Error,
	Progress
}


pub struct Notification {
	notification_type: NotificationType,
	pub group: Option<String>,
	pub title: Option<String>,
	pub content: Option<String>,
	pub id: Option<String>,

	//onclick_app: Option<String>,
	// onclick_file: entry_ref,
	// onclick_refs: Vec<entry_ref>,
	//onclick_args: Vec<String>,
	// icon
	source_signature: String,
	source_name: String,
	progress: f32,
}


impl Default for Notification {
	fn default() -> Self {
		// get app info
		let (team, _) = get_current_team_and_thread();
		let info = ROSTER.get_running_app_info(&Team::from(team).unwrap()).unwrap();
		let filename = match Path::new(&info.path).file_name() {
			Some(file) => String::from(file.to_str().unwrap()),
			None => String::new()
		};

		Notification {
			notification_type: NotificationType::Information,
			group: None,
			title: None,
			content: None,
			id: None,
			// onclick_app: String::new(),
			// onclick_file,
			// onclick_refs: Vec::new(),
			// onclick_args: Vec::new(),
			// icon,
			source_signature: info.signature,
			source_name: filename,
			progress: 0.0
		}
	}
}


impl Notification {
	fn to_message(&self) -> Result<Message> {
		let mut message = Message::new(NOTIFICATION_MESSAGE);
		message.add_data("_appname", &self.source_name);
		message.add_data("_signature", &self.source_signature);
		let type_: i32 = match &self.notification_type {
			NotificationType::Information => 0,
			NotificationType::Important => 1,
			NotificationType::Error => 2,
			NotificationType::Progress => 3
		};
		message.add_data("_type", &type_);
		self.group.as_ref().map(|group| message.add_data("_group", group));
		self.title.as_ref().map(|title| message.add_data("_title", title));
		self.content.as_ref().map(|content| message.add_data("_content", content));
		self.id.as_ref().map(|id| message.add_data("_messageID", id));
		if self.notification_type == NotificationType::Progress {
			message.add_data("_progress", &self.progress);
		}

		// message.add_data("_onClickApp"
		// message.add_data("_onClickFile"
		// message.add_data("_onClickRef"
		// message.add_data("_onClickArgv"
		// message.add_data("_icon"
		Ok(message)
	}

	pub fn send(&self, replyto: &Messenger, duration: Option<Duration>) -> Result<()> {
		let mut message = self.to_message()?;
		let timeout_ms: i64 = match duration {
			Some(d) => d.as_secs() as i64 * 1_000_000 + d.subsec_micros() as i64,
			None => 0
		};
		if timeout_ms > 0 {
			message.add_data("timeout", &timeout_ms);
		}
		let mut messenger = Messenger::from_signature(NOTIFICATION_SERVER_SIGNATURE, None)?;
		messenger.send(message, replyto)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use app::{Application, ApplicationDelegate, ApplicationHooks, Message, Messenger, QUIT};
	use super::*;

	struct MockApplicationState { }
	impl ApplicationHooks for MockApplicationState {
		fn message_received(&mut self, application: &ApplicationDelegate, message: &Message) {
		}
		
		fn ready_to_run(&mut self, application: &ApplicationDelegate) {
			let notification = Notification {
				title: Some(String::from("Information")), 
				content: Some(String::from("This notification comes from Rust")),
				.. Default::default()
			};
			notification.send(&application.messenger, None);
			application.quit();
		}
	}

	const MOCK_SIGNATURE: &str = "application/notification_test";

	#[test]
	fn test_notification() {
		let mut application = Application::new(MOCK_SIGNATURE, MockApplicationState {});
		application.run();
	}
}
