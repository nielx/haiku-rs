//
// Copyright 2019, 2024, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::CStr;
use std::mem;
use std::path::PathBuf;

use libc::c_char;
use libc::B_OK;
use libc::{area_id, get_next_image_info, image_info, image_type, port_id, team_id, type_code};

use haiku_constant;
use support::{ErrorKind, HaikuError, Result};

// os/app/AppDefs.h
pub const B_ARGV_RECEIVED: u32 = haiku_constant!('_', 'A', 'R', 'G');
pub const B_READY_TO_RUN: u32 = haiku_constant!('_', 'R', 'T', 'R');
pub const B_QUIT_REQUESTED: u32 = haiku_constant!('_', 'Q', 'R', 'Q');
pub const QUIT: u32 = haiku_constant!('_', 'Q', 'I', 'T');

// private/app/MessagePrivate.h
pub const MESSAGE_FLAG_VALID: u32 = 0x0001;
pub const MESSAGE_FLAG_REPLY_REQUIRED: u32 = 0x0002;
pub const MESSAGE_FLAG_REPLY_DONE: u32 = 0x0004;
pub const MESSAGE_FLAG_IS_REPLY: u32 = 0x0008;
pub const MESSAGE_FLAG_WAS_DELIVERED: u32 = 0x0010;
pub const MESSAGE_FLAG_HAS_SPECIFIERS: u32 = 0x0020;
pub const MESSAGE_FLAG_WAS_DROPPED: u32 = 0x0040;
pub const MESSAGE_FLAG_PASS_BY_AREA: u32 = 0x0080;
pub const MESSAGE_FLAG_REPLY_AS_KMESSAGE: u32 = 0x0100;

pub const FIELD_FLAG_VALID: u16 = 0x0001;
pub const FIELD_FLAG_FIXED_SIZE: u16 = 0x0002;

pub const MESSAGE_FORMAT_HAIKU: u32 = haiku_constant!('1', 'F', 'M', 'H');

// private/app/TokenSpace.h
pub const B_PREFERRED_TOKEN: i32 = -2;
pub const B_NULL_TOKEN: i32 = -1;
pub const B_ANY_TOKEN: i32 = 0;
pub const B_HANDLER_TOKEN: i32 = 1;
pub const B_SERVER_TOKEN: i32 = 2;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct field_header {
	pub flags: u16,
	pub name_length: u16,
	pub field_type: type_code, // The original name 'type' is reserved
	pub count: u32,
	pub data_size: u32,
	pub offset: u32,
	pub next_field: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct message_header {
	pub message_format: u32,
	pub what: u32,
	pub flags: u32,

	pub target: i32,
	pub current_specifier: i32,
	pub message_area: area_id,

	pub reply_port: port_id,
	pub reply_target: i32,
	pub reply_team: team_id,

	pub data_size: u32,
	pub field_count: u32,
	pub hash_table_size: u32,
	pub hash_table: [i32; 5],
}

// Helper functions
pub(crate) fn get_app_path(team: team_id) -> Result<PathBuf> {
	let mut info = mem::MaybeUninit::<image_info>::uninit();
	let mut cookie: i32 = 0;

	// Initial run to initialize memory
	let mut result = unsafe { get_next_image_info(team, &mut cookie, info.as_mut_ptr()) };
	if result != B_OK {
		return Err(HaikuError::new(
			ErrorKind::NotFound,
			"Cannot find the app image",
		));
	}
	let mut info = unsafe { info.assume_init() };

	// Iterate over the rest of the images until the app image is found
	while result == B_OK {
		if info.image_type == image_type::B_APP_IMAGE as i32 {
			let c_name = unsafe { CStr::from_ptr((&info.name) as *const c_char) };
			return Ok(PathBuf::from(c_name.to_str().unwrap()));
		}
		result = unsafe { get_next_image_info(team, &mut cookie, &mut info) };
	}

	Err(HaikuError::new(
		ErrorKind::NotFound,
		"Cannot find the app image",
	))
}
