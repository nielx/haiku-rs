//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

// This module contains the private messaging interface between Haiku applications
// and the app server.

#[repr(C)]
struct message_header {
	size: i32,
	code: u32,
	flags: u32
}

const LINK_CODE: i32 = haiku_constant!('_','P','T','L') as i32;
const INITIAL_BUFFER_SIZE: usize = 2048;
const MAX_BUFFER_SIZE: usize = 65536;
const NEEDS_REPLY: u32 = 0x01;
