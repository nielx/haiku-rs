//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]

use ::{type_code, area_id, port_id, team_id};

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

#[repr(C)]
pub struct field_header {
	flags: u16,
	name_length: u16,
	field_type: type_code, // The original name 'type' is reserved
	count: u32,
	data_size: u32,
	offset: u32,
	next_field: i32
}

#[repr(C)]
pub struct message_header {
	format: u32,
	what: u32,
	flags: u32,
	
	target: i32,
	current_specifier: i32,
	message_area: area_id,
	
	reply_port: port_id,
	reply_target: i32,
	reply_team: team_id,
	
	data_size: u32,
	field_count: u32,
	hash_table_size: u32,
	hash_table: [i32; 5]
}
