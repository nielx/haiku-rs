//
// Copyright 2018-2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

//! The application kit contains tools to structure your application and to
//! communicate with other applications and services


mod message;
mod messenger;
mod roster;
pub(crate) mod sys;

pub use self::message::Message;
pub use self::messenger::Messenger;
pub use self::roster::{ROSTER, AppInfo, Roster};
