//
// Copyright 2018-2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

//! The application kit contains tools to structure your application and to
//! communicate with other applications and services


mod application;
mod looper;
mod message;
mod messenger;
mod notification;
mod roster;
pub(crate) mod serverlink;
pub(crate) mod sys;

pub use self::application::{Application, ApplicationDelegate, ApplicationHooks, Context};
pub use self::looper::{Handler, Looper, LooperDelegate};
pub use self::message::Message;
pub use self::messenger::Messenger;
pub use self::notification::{Notification, NotificationType};
pub use self::roster::{ROSTER, AppInfo, Roster};

pub const B_ARGV_RECEIVED: u32 = haiku_constant!('_','A','R','G');
pub const B_READY_TO_RUN: u32 = haiku_constant!('_','R','T','R');
pub const B_QUIT_REQUESTED: u32 = haiku_constant!('_','Q','R','Q');
pub const QUIT: u32 = haiku_constant!('_','Q','I','T');
