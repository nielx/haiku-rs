//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]

use haiku_sys::{type_code, area_id, port_id, team_id};

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

pub const MESSAGE_FORMAT_HAIKU: u32 = haiku_constant!('1','F','M','H');

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
	pub next_field: i32
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
	pub hash_table: [i32; 5]
}
