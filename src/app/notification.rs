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
/// The type of notification
///
/// The notification type currently influences the look of the notification.
/// In particular the look of the `Progress` notification includes a progress
/// bar, with a configurable fill state.
pub enum NotificationType {
	/// Information notification
	///
	/// This type of notification has a grey sidebar.
	Information,
	/// Important notification
	///
	/// This type of notification has a blue sidebar.
	Important,
	/// Error notification
	///
	/// This type of notification has a red sidebar.
	Error,
	/// Progress notification
	///
	/// This type of notification includes a progress bar.
	Progress
}

/// Notification for Haiku's notification system.
///
/// In order to create and send a notification for Haiku's general notification
/// server, you create an object from this class and set the parameters that
/// you want to tweak.
///
/// By default all parameters are optional and have a default value.
///
/// # Example
///
/// ```norun
/// # extern crate haiku;
/// # use haiku::app::{Messenger, Notification, NotificationType};
/// # use std::time::Duration;
/// # let reply_to_messenger = Messenger::from_port_id(-1);
/// let notification = Notification {
/// 	notification_type: NotificationType::Progress,
/// 	title: Some(String::from("My Progress")),
/// 	content: Some(String::from("Updating Something")),
/// 	progress: 0.5,
/// 	.. Default::default()
/// };
/// 
/// notification.send(&reply_to_messenger, Some(Duration::new(5, 0)));
/// ```
pub struct Notification {
	/// The type of notification
	///
	/// This is `NotificationType::Information` by default.
	pub notification_type: NotificationType,

	/// The name of a group 
	///
	/// The notification system will position notifications that share the
	/// same group name.
	pub group: Option<String>,

	/// The title
	pub title: Option<String>,

	/// The message of the notification
	pub content: Option<String>,

	/// A unique identifier for the notification
	///
	/// Setting an identifier allows you to replace a current notification
	/// with an updated version. This one is particularly useful in conjunction
	/// with a progress notification.
	pub id: Option<String>,

	/// A floating point that determines how full a progress bar is
	///
	/// This option only has effect on the NotificationType::Progress, which
	/// displays a progress bar.
	/// The value you enter needs to be between 0.0 and 1.0. Any value below
	/// 0.0 will lead to 0.0 as value, and any value above 1.0 will lead to
	/// 1.0 being set.
	pub progress: f32,


	// TODO: onclick_app: Option<String>,
	// TODO: onclick_file: entry_ref,
	// TODO: onclick_refs: Vec<entry_ref>,
	// TODO: onclick_args: Vec<String>,
	// TODO: icon
	source_signature: String,
	source_name: String,
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
			progress: 0.0,
			// onclick_app: String::new(),
			// onclick_file,
			// onclick_refs: Vec::new(),
			// onclick_args: Vec::new(),
			// icon,
			source_signature: info.signature,
			source_name: filename,
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

		if self.notification_type == NotificationType::Progress {
			let progress = if self.progress < 0.0 {
					0.0
				} else if self.progress > 1.0 {
					1.0
				} else {
					self.progress
				};
			message.add_data("_progress", &progress);
		}
		// TODO: message.add_data("_onClickApp"
		// TODO: message.add_data("_onClickFile"
		// TODO: message.add_data("_onClickRef"
		// TODO: message.add_data("_onClickArgv"
		// TODO: message.add_data("_icon"
		Ok(message)
	}

	/// Send the notification to the system to display it
	///
	/// It is possible to add a duration. This will override the default
	/// display duration.
	///
	/// You retain ownership of the notification after this has been sent. This
	/// will allow you to modify and resend it. Especially in the case of
	/// progress notifications this may be useful.
	pub fn send(&self, replyto: &Messenger, duration: Option<Duration>) -> Result<()> {
		let mut message = self.to_message()?;
		let timeout_ms: i64 = match duration {
			Some(d) => d.as_secs() as i64 * 1_000_000 + d.subsec_micros() as i64,
			None => 0
		};
		if timeout_ms > 0 {
			message.add_data("timeout", &timeout_ms);
		}
		let messenger = Messenger::from_signature(NOTIFICATION_SERVER_SIGNATURE, None)?;
		messenger.send(message, replyto)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use app::{Application, ApplicationDelegate, ApplicationHooks};
	use super::*;

	struct MockApplicationState { }
	impl ApplicationHooks for MockApplicationState {
		fn ready_to_run(&mut self, application: &ApplicationDelegate) {
			let notification = Notification {
				title: Some(String::from("Information")), 
				content: Some(String::from("This notification comes from Rust")),
				.. Default::default()
			};
			notification.send(&application.messenger, None).unwrap();
			application.quit();
		}
	}

	const MOCK_SIGNATURE: &str = "application/notification_test";

	#[test]
	fn test_notification() {
		let application = Application::new(MOCK_SIGNATURE, MockApplicationState {});
		application.run().unwrap();
	}
}
