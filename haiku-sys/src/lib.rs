//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

#![allow(non_camel_case_types)]

extern crate libc;
use libc::{c_int, c_char, DIR, dirent, FILENAME_MAX, off_t, PATH_MAX, size_t, ssize_t};
use std::mem;

#[macro_export]
macro_rules! haiku_constant {
	($a:tt, $b:tt, $c:tt, $d:tt) => ((($a as u32) << 24) + (($b as u32) << 16) + (($c as u32) << 8) + ($d as u32));
}

pub mod message;

// OS.h
pub const B_OS_NAME_LENGTH : usize = 32;
pub const B_TIMEOUT: u32 = 0x8;

pub type area_id = i32;
pub type port_id = i32;
pub type sem_id = i32;
pub type team_id = i32;
pub type thread_id = i32;

pub type status_t = i32;
pub type bigtime_t = i64;

#[repr(C)]
pub struct port_info {
	pub port: port_id,
	pub team: team_id,
	pub name: [c_char; B_OS_NAME_LENGTH],
	pub capacity: i32,
	pub queue_count: i32,
	pub total_count: i32,
}

extern {
	pub fn create_port(capacity: i32, name: *const c_char) -> port_id;
	pub fn find_port(name: *const c_char) -> port_id;
	pub fn read_port(port: port_id, code: *mut i32, buffer: *mut u8,
										bufferSize: size_t) -> ssize_t;
	pub fn read_port_etc(port: port_id, code: *mut i32, buffer: *mut u8,
										bufferSize: size_t, flags: u32,
										timeout: bigtime_t) -> ssize_t;
	pub fn write_port(port: port_id, code: i32, buffer: *const u8,
										bufferSize: size_t) -> status_t;
	pub fn write_port_etc(port: port_id, code: i32, buffer: *const u8,
										bufferSize: size_t, flags: u32,
										timeout: bigtime_t) -> status_t;
	pub fn close_port(port: port_id) -> status_t;
	pub fn delete_port(port: port_id) -> status_t;
	pub fn port_buffer_size(port: port_id) -> ssize_t;
	pub fn port_buffer_size_etc(port: port_id, flags: u32, 
										timeout: bigtime_t) -> ssize_t;
	pub fn port_count(port: port_id) -> ssize_t;
	// set_port_owner
	
	fn _get_port_info(port: port_id, buf: *mut port_info,
		              portInfoSize: size_t) -> status_t;
	// _get_next_port_info 
}

pub unsafe fn get_port_info(port: port_id, buf: &mut port_info) -> status_t {
	_get_port_info(port, buf, mem::size_of::<port_info>() as size_t)
}


// fs_attr.h
#[repr(C)]
pub struct attr_info {
	pub attr_type: u32,
	pub size: off_t,
}

extern {
	pub fn fs_read_attr(fd: c_int, attribute: *const c_char, typeCode: u32,
						pos: off_t, buffer: *mut u8, readBytes: size_t) -> ssize_t;
	pub fn fs_write_attr(fd: c_int, attribute: *const c_char, typeCode: u32,
						pos: off_t, buffer: *const u8, readBytes: size_t) -> ssize_t;
	pub fn fs_remove_attr(fd: c_int, attribute: *const c_char) -> c_int;
	pub fn fs_stat_attr(fd: c_int, attribute: *const c_char, attrInfo: *mut attr_info) -> c_int;
	
	pub fn fs_open_attr(path: *const c_char, attribute: *const c_char,
						typeCode: u32, openMode: c_int) -> c_int;
	pub fn fs_fopen_attr(fd: c_int, attribute: *const c_char, typeCode: u32, 
						openMode: c_int) -> c_int;
	pub fn fs_close_attr(fd: c_int) -> c_int;
	
	pub fn fs_open_attr_dir(path: *const c_char) -> *mut DIR;
	pub fn fs_lopen_attr_dir(path: *const c_char) -> *mut DIR;
	pub fn fs_fopen_attr_dir(fd: c_int) -> *mut DIR;
	pub fn fs_close_attr_dir(dir: *mut DIR) -> c_int;
	pub fn fs_read_attr_dir(dir: *mut DIR) -> *mut dirent;
	pub fn fs_rewind_attr_dir(dir: *mut DIR);
}

// support/TypeConstants.h
pub const B_AFFINE_TRANSFORM_TYPE: u32 = haiku_constant!('A','M','T','X');
pub const B_ALIGNMENT_TYPE: u32 = haiku_constant!('A','L','G','N');
pub const B_ANY_TYPE: u32 = haiku_constant!('A','N','Y','T');
pub const B_ATOM_TYPE: u32 = haiku_constant!('A','T','O','M');
pub const B_ATOM_REF_TYPE: u32 = haiku_constant!('A','T','M','R');
pub const B_BOOL_TYPE: u32 = haiku_constant!('B','O','O','L');
pub const B_CHAR_TYPE: u32 = haiku_constant!('C','H','A','R');
pub const B_COLOR_8_BIT_TYPE: u32 = haiku_constant!('C','L','R','B');
pub const B_DOUBLE_TYPE: u32 = haiku_constant!('D','B','L','E');
pub const B_FLOAT_TYPE: u32 = haiku_constant!('F','L','O','T');
pub const B_GRAYSCALE_8_BIT_TYPE: u32 = haiku_constant!('G','R','Y','B');
pub const B_INT16_TYPE: u32 = haiku_constant!('S','H','R','T');
pub const B_INT32_TYPE: u32 = haiku_constant!('L','O','N','G');
pub const B_INT64_TYPE: u32 = haiku_constant!('L','L','N','G');
pub const B_INT8_TYPE: u32 = haiku_constant!('B','Y','T','E'); 
pub const B_LARGE_ICON_TYPE: u32 = haiku_constant!('I','C','O','N');
pub const B_MEDIA_PARAMETER_GROUP_TYPE: u32 = haiku_constant!('B','M','C','G');
pub const B_MEDIA_PARAMETER_TYPE: u32 = haiku_constant!('B','M','C','T');
pub const B_MEDIA_PARAMETER_WEB_TYPE: u32 = haiku_constant!('B','M','C','W');
pub const B_MESSAGE_TYPE: u32 = haiku_constant!('M','S','G','G');
pub const B_MESSENGER_TYPE: u32 = haiku_constant!('M','S','N','G');
pub const B_MIME_TYPE: u32 = haiku_constant!('M','I','M','E');
pub const B_MINI_ICON_TYPE: u32 = haiku_constant!('M','I','C','N');
pub const B_MONOCHROME_1_BIT_TYPE: u32 = haiku_constant!('M','N','O','B');
pub const B_OBJECT_TYPE: u32 = haiku_constant!('O','P','T','R');
pub const B_OFF_T_TYPE: u32 = haiku_constant!('O','F','F','T');
pub const B_PATTERN_TYPE: u32 = haiku_constant!('P','A','T','N');
pub const B_POINTER_TYPE: u32 = haiku_constant!('P','N','T','R');
pub const B_POINT_TYPE: u32 = haiku_constant!('B','P','N','T');
pub const B_PROPERTY_INFO_TYPE: u32 = haiku_constant!('S','C','T','D');
pub const B_RAW_TYPE: u32 = haiku_constant!('R','A','W','T');
pub const B_RECT_TYPE: u32 = haiku_constant!('R','E','C','T');
pub const B_REF_TYPE: u32 = haiku_constant!('R','R','E','F');
pub const B_RGB_32_BIT_TYPE: u32 = haiku_constant!('R','G','B','B');
pub const B_RGB_COLOR_TYPE: u32 = haiku_constant!('R','G','B','C');
pub const B_SIZE_TYPE: u32 = haiku_constant!('S','I','Z','E');
pub const B_SIZE_T_TYPE: u32 = haiku_constant!('S','I','Z','T');
pub const B_SSIZE_T_TYPE: u32 = haiku_constant!('S','S','Z','T');
pub const B_STRING_TYPE: u32 = haiku_constant!('C','S','T','R');
pub const B_STRING_LIST_TYPE: u32 = haiku_constant!('S','T','R','L');
pub const B_TIME_TYPE: u32 = haiku_constant!('T','I','M','E');
pub const B_UINT16_TYPE: u32 = haiku_constant!('U','S','H','T');
pub const B_UINT32_TYPE: u32 = haiku_constant!('U','L','N','G');
pub const B_UINT64_TYPE: u32 = haiku_constant!('U','L','L','G');
pub const B_UINT8_TYPE: u32 = haiku_constant!('U','B','Y','T');
pub const B_VECTOR_ICON_TYPE: u32 = haiku_constant!('V','I','C','N');
pub const B_XATTR_TYPE: u32 = haiku_constant!('X','A','T','R');
pub const B_NETWORK_ADDRESS_TYPE: u32 = haiku_constant!('N','W','A','D');
pub const B_MIME_STRING_TYPE: u32 = haiku_constant!('M','I','M','S');

// SupportDefs.h
pub type type_code = u32;

// StorageDefs.h
pub const B_DEV_NAME_LENGTH: usize = 128;
pub const B_FILE_NAME_LENGTH: usize = FILENAME_MAX as usize;
pub const B_PATH_NAME_LENGTH: usize = PATH_MAX as usize;
pub const B_ATTR_NAME_LENGTH: usize = B_FILE_NAME_LENGTH - 1;
pub const B_MIME_TYPE_LENGTH: usize = B_ATTR_NAME_LENGTH - 15;
// Todo: SYMLOOP_MAX, needs to come from libc


// Errors.h
pub mod errors {
	use std;
	use libc::{EOVERFLOW, E2BIG, EFBIG, ERANGE, ENODEV, EOPNOTSUPP};
	use ::status_t;
	
	pub const B_GENERAL_ERROR_BASE: status_t = std::i32::MIN;
	pub const B_OS_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x1000;
	pub const B_APP_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x2000;
	pub const B_INTERFACE_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x3000;
	pub const B_MEDIA_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x4000;
	pub const B_TRANSLATION_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x4800;
	pub const B_MIDI_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x5000;
	pub const B_STORAGE_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x6000;
	pub const B_POSIX_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x7000;
	pub const B_MAIL_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x8000;
	pub const B_PRINT_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0x9000;
	pub const B_DEVICE_ERROR_BASE: status_t = B_GENERAL_ERROR_BASE + 0xa000;
	pub const B_ERRORS_END: status_t = B_GENERAL_ERROR_BASE + 0xffff;

	// General errors
	pub const B_NO_MEMORY: status_t = B_GENERAL_ERROR_BASE + 0;
	pub const B_IO_ERROR: status_t = B_GENERAL_ERROR_BASE + 1;
	pub const B_PERMISSION_DENIED: status_t = B_GENERAL_ERROR_BASE + 2;
	pub const B_BAD_INDEX: status_t = B_GENERAL_ERROR_BASE + 3;
	pub const B_BAD_TYPE: status_t = B_GENERAL_ERROR_BASE + 4;
	pub const B_BAD_VALUE: status_t = B_GENERAL_ERROR_BASE + 5;
	pub const B_MISMATCHED_VALUES: status_t = B_GENERAL_ERROR_BASE + 6;
	pub const B_NAME_NOT_FOUND: status_t = B_GENERAL_ERROR_BASE + 7;
	pub const B_NAME_IN_USE: status_t = B_GENERAL_ERROR_BASE + 8;
	pub const B_TIMED_OUT: status_t = B_GENERAL_ERROR_BASE + 9;
	pub const B_INTERRUPED: status_t = B_GENERAL_ERROR_BASE + 10;
	pub const B_WOULD_BLOCK: status_t = B_GENERAL_ERROR_BASE + 11;
	pub const B_CANCELED: status_t = B_GENERAL_ERROR_BASE + 12;
	pub const B_NO_INIT: status_t = B_GENERAL_ERROR_BASE + 13;
	pub const B_NOT_INITIALIZED: status_t = B_GENERAL_ERROR_BASE + 13;
	pub const B_BUSY: status_t = B_GENERAL_ERROR_BASE + 14;
	pub const B_NOT_ALLOWED: status_t = B_GENERAL_ERROR_BASE + 15;
	pub const B_BAD_DATA: status_t = B_GENERAL_ERROR_BASE + 16;
	pub const B_DONT_DO_THAT: status_t = B_GENERAL_ERROR_BASE + 17;

	pub const B_ERROR: status_t = -1;
	pub const B_OK: status_t = 0;
	pub const B_NO_ERROR: status_t = 0;

	// Kernel kit errors
	pub const B_BAD_SEM_ID: status_t = B_OS_ERROR_BASE + 0;
	pub const B_NO_MORE_SEMS: status_t = B_OS_ERROR_BASE + 1;

	pub const B_BAD_THREAD_ID: status_t = B_OS_ERROR_BASE + 0x100;
	pub const B_NO_MORE_THREADS: status_t = B_OS_ERROR_BASE + 0x101;
	pub const B_BAD_THREAD_STATE: status_t = B_OS_ERROR_BASE + 0x012;
	pub const B_BAD_TEAM_ID: status_t = B_OS_ERROR_BASE + 0x103;
	pub const B_NO_MORE_TEAMS: status_t = B_OS_ERROR_BASE + 0x104;

	pub const B_BAD_PORT_ID: status_t = B_OS_ERROR_BASE + 0x200;
	pub const B_NO_MORE_PORTS: status_t = B_OS_ERROR_BASE + 0x201;

	pub const B_BAD_IMAGE_ID: status_t = B_OS_ERROR_BASE + 0x300;
	pub const B_BAD_ADDRESS: status_t = B_OS_ERROR_BASE + 0x301;
	pub const B_NOT_AN_EXECUTABLE: status_t = B_OS_ERROR_BASE + 0x302;
	pub const B_MISSING_LIBRARY: status_t = B_OS_ERROR_BASE + 0x303;
	pub const B_MISSING_SYMBOL: status_t = B_OS_ERROR_BASE + 0x304;
	pub const B_UNKNOWN_EXECUTABLE: status_t = B_OS_ERROR_BASE + 0x305;
	pub const B_LEGACY_EXECUTABLE: status_t = B_OS_ERROR_BASE + 0x306;

	pub const B_DEBUGGER_ALREADY_INSTALLED: status_t = B_OS_ERROR_BASE + 0x400;

	// Application kit errors
	pub const B_BAD_REPLY: status_t = B_APP_ERROR_BASE + 0;
	pub const B_DUPLICATE_REPLY: status_t = B_APP_ERROR_BASE + 1;
	pub const B_MESSAGE_TO_SELF: status_t = B_APP_ERROR_BASE + 2;
	pub const B_BAD_HANDLER: status_t = B_APP_ERROR_BASE + 3;
	pub const B_ALREADY_RUNNING: status_t = B_APP_ERROR_BASE + 4;
	pub const B_LAUNCH_FAILED: status_t = B_APP_ERROR_BASE + 5;
	pub const B_AMBIGUOUS_APP_LAUNCH: status_t = B_APP_ERROR_BASE + 6;
	pub const B_UNKNOWN_MIME_TYPE: status_t = B_APP_ERROR_BASE + 7;
	pub const B_BAD_SCRIPT_SYNTAX: status_t = B_APP_ERROR_BASE + 8;
	pub const B_LAUNCH_FAILED_NO_RESOLVE_LINK: status_t = B_APP_ERROR_BASE + 9;
	pub const B_LAUNCH_FAILED_EXECUTABLE: status_t = B_APP_ERROR_BASE + 10;
	pub const B_LAUNCH_FAILED_APP_NOT_FOUND: status_t = B_APP_ERROR_BASE + 11;
	pub const B_LAUNCH_FAILED_APP_IN_TRASH: status_t = B_APP_ERROR_BASE + 12;
	pub const B_LAUNCH_FAILED_NO_PREFERRED_APP: status_t = B_APP_ERROR_BASE + 13;
	pub const B_LAUNCH_FAILED_FILES_APP_NOT_FOUND: status_t = B_APP_ERROR_BASE + 14;
	pub const B_BAD_MIME_SNIFFER_RULE: status_t = B_APP_ERROR_BASE + 15;
	pub const B_NOT_A_MESSAGE: status_t = B_APP_ERROR_BASE + 16;
	pub const B_SHUTDOWN_CANCELLED: status_t = B_APP_ERROR_BASE + 17;
	pub const B_SHUTTING_DOWN: status_t = B_APP_ERROR_BASE + 18;

	// Storage kit errors
	pub const B_FILE_ERROR: status_t = B_STORAGE_ERROR_BASE + 0;
	pub const B_FILE_NOT_FOUND: status_t = B_STORAGE_ERROR_BASE + 1;
	pub const B_FILE_EXISTS: status_t = B_STORAGE_ERROR_BASE + 2;
	pub const B_ENTRY_NOT_FOUND: status_t = B_STORAGE_ERROR_BASE + 3;
	pub const B_NAME_TOO_LONG: status_t = B_STORAGE_ERROR_BASE + 4;
	pub const B_NOT_A_DIRECTORY: status_t = B_STORAGE_ERROR_BASE + 5;
	pub const B_DIRECTORY_NOT_EMPTY: status_t = B_STORAGE_ERROR_BASE + 6;
	pub const B_DEVICE_FULL: status_t = B_STORAGE_ERROR_BASE + 7;
	pub const B_READ_ONLY_DEVICE: status_t = B_STORAGE_ERROR_BASE + 8;
	pub const B_IS_A_DIRECTORY: status_t = B_STORAGE_ERROR_BASE + 9;
	pub const B_NO_MORE_FDS: status_t = B_STORAGE_ERROR_BASE + 10;
	pub const B_CROSS_DEVICE_LINK: status_t = B_STORAGE_ERROR_BASE + 11;
	pub const B_LINK_LIMIT: status_t = B_STORAGE_ERROR_BASE + 12;
	pub const B_BUSTED_PIPE: status_t = B_STORAGE_ERROR_BASE + 13;
	pub const B_UNSUPPORTED: status_t = B_STORAGE_ERROR_BASE + 14;
	pub const B_PARTITION_TOO_SMALL: status_t = B_STORAGE_ERROR_BASE + 15;
	pub const B_PARTIAL_READ: status_t = B_STORAGE_ERROR_BASE + 16;
	pub const B_PARTIAL_WRITE: status_t = B_STORAGE_ERROR_BASE + 17;

	// Mapped posix errors
	pub const B_BUFFER_OVERFLOW: status_t = EOVERFLOW;
	pub const B_TOO_MANY_ARGS: status_t = E2BIG;
	pub const B_FILE_TOO_LARGE: status_t = EFBIG;
	pub const B_RESULT_NOT_REPRESENTABLE: status_t = ERANGE;
	pub const B_DEVICE_NOT_FOUND: status_t = ENODEV;
	pub const B_NOT_SUPPORTED: status_t = EOPNOTSUPP;

	// Media kit errors
	pub const B_STREAM_NOT_FOUND: status_t = B_MEDIA_ERROR_BASE + 0;
	pub const B_SERVER_NOT_FOUND: status_t = B_MEDIA_ERROR_BASE + 1;
	pub const B_RESOURCE_NOT_FOUND: status_t = B_MEDIA_ERROR_BASE + 2;
	pub const B_RESOURCE_UNAVAILABLE: status_t = B_MEDIA_ERROR_BASE + 3;
	pub const B_BAD_SUBSCRIBER: status_t = B_MEDIA_ERROR_BASE + 4;
	pub const B_SUBSCRIBER_NOT_ENTERED: status_t = B_MEDIA_ERROR_BASE + 5;
	pub const B_BUFFER_NOT_AVAILABLE: status_t = B_MEDIA_ERROR_BASE + 6;
	pub const B_LAST_BUFFER_ERROR: status_t = B_MEDIA_ERROR_BASE + 7;

	pub const B_MEDIA_SYSTEM_FAILURE: status_t = B_MEDIA_ERROR_BASE + 100;
	pub const B_MEDIA_BAD_NODE: status_t = B_MEDIA_ERROR_BASE + 101;
	pub const B_MEDIA_NODE_BUSY: status_t = B_MEDIA_ERROR_BASE + 102;
	pub const B_MEDIA_BAD_FORMAT: status_t = B_MEDIA_ERROR_BASE + 103;
	pub const B_MEDIA_BAD_BUFFER: status_t = B_MEDIA_ERROR_BASE + 104;
	pub const B_MEDIA_TOO_MANY_NODES: status_t = B_MEDIA_ERROR_BASE + 105;
	pub const B_MEDIA_TOO_MANY_BUFFERS: status_t = B_MEDIA_ERROR_BASE + 106;
	pub const B_MEDIA_NODE_ALREADY_EXISTS: status_t = B_MEDIA_ERROR_BASE + 107;
	pub const B_MEDIA_BUFFER_ALREADY_EXISTS: status_t = B_MEDIA_ERROR_BASE + 108;
	pub const B_MEDIA_CANNOT_SEEK: status_t = B_MEDIA_ERROR_BASE + 109;
	pub const B_MEDIA_CANNOT_CHANGE_RUN_MODE: status_t = B_MEDIA_ERROR_BASE + 110;
	pub const B_MEDIA_APP_ALREADY_REGISTERED: status_t = B_MEDIA_ERROR_BASE + 111;
	pub const B_MEDIA_APP_NOT_REGISTERED: status_t = B_MEDIA_ERROR_BASE + 112;
	pub const B_MEDIA_CANNOT_RECLAIM_BUFFERS: status_t = B_MEDIA_ERROR_BASE + 113;
	pub const B_MEDIA_BUFFERS_NOT_RECLAIMED: status_t = B_MEDIA_ERROR_BASE + 114;
	pub const B_MEDIA_TIME_SOURCE_STOPPED: status_t = B_MEDIA_ERROR_BASE + 115;
	pub const B_MEDIA_TIME_SOURCE_BUSY: status_t = B_MEDIA_ERROR_BASE + 116;
	pub const B_MEDIA_BAD_SOURCE: status_t = B_MEDIA_ERROR_BASE + 117;
	pub const B_MEDIA_BAD_DESTINATION: status_t = B_MEDIA_ERROR_BASE + 118;
	pub const B_MEDIA_ALREADY_CONNECTED: status_t = B_MEDIA_ERROR_BASE + 119;
	pub const B_MEDIA_NOT_CONNECTED: status_t = B_MEDIA_ERROR_BASE + 120;
	pub const B_MEDIA_BAD_CLIP_FORMAT: status_t = B_MEDIA_ERROR_BASE + 121;
	pub const B_MEDIA_ADDON_FAILED: status_t = B_MEDIA_ERROR_BASE + 122;
	pub const B_MEDIA_ADDON_DISABLED: status_t = B_MEDIA_ERROR_BASE + 123;
	pub const B_MEDIA_CHANGE_IN_PROGRESS: status_t = B_MEDIA_ERROR_BASE + 124;
	pub const B_MEDIA_STALE_CHANGE_COUNT: status_t = B_MEDIA_ERROR_BASE + 125;
	pub const B_MEDIA_ADDON_RESTRICTED: status_t = B_MEDIA_ERROR_BASE + 126;
	pub const B_MEDIA_NO_HANDLER: status_t = B_MEDIA_ERROR_BASE + 127;
	pub const B_MEDIA_DUPLICATE_FORMAT: status_t = B_MEDIA_ERROR_BASE + 128;
	pub const B_MEDIA_REALTIME_DISABLED: status_t = B_MEDIA_ERROR_BASE + 129;
	pub const B_MEDIA_REALTIME_UNAVAILABLE: status_t = B_MEDIA_ERROR_BASE + 130;

	// Mail kit errors
	pub const B_MAIL_NO_DAEMON: status_t = B_MAIL_ERROR_BASE + 0;
	pub const B_MAIL_UNKNOWN_USER: status_t = B_MAIL_ERROR_BASE + 1;
	pub const B_MAIL_WRONG_PASSWORD: status_t = B_MAIL_ERROR_BASE + 2;
	pub const B_MAIL_UNKNOWN_HOST: status_t = B_MAIL_ERROR_BASE + 3;
	pub const B_MAIL_ACCESS_ERROR: status_t = B_MAIL_ERROR_BASE + 4;
	pub const B_MAIL_UNKNOWN_FIELD: status_t = B_MAIL_ERROR_BASE + 5;
	pub const B_MAIL_NO_RECIPIENT: status_t = B_MAIL_ERROR_BASE + 6;
	pub const B_MAIL_INVALID_MAIL: status_t = B_MAIL_ERROR_BASE + 7;

	// Print kit errors
	pub const B_NO_PRINT_SERVER: status_t = B_PRINT_ERROR_BASE + 0;

	// Device kit errors
	pub const B_DEV_INVALID_IOCTL: status_t = B_DEVICE_ERROR_BASE + 0;
	pub const B_DEV_NO_MEMORY: status_t = B_DEVICE_ERROR_BASE + 1;
	pub const B_DEV_BAD_DRIVE_NUM: status_t = B_DEVICE_ERROR_BASE + 2;
	pub const B_DEV_NO_MEDIA: status_t = B_DEVICE_ERROR_BASE + 3;
	pub const B_DEV_UNREADABLE: status_t = B_DEVICE_ERROR_BASE + 4;
	pub const B_DEV_FORMAT_ERROR: status_t = B_DEVICE_ERROR_BASE + 5;
	pub const B_DEV_TIMEOUT: status_t = B_DEVICE_ERROR_BASE + 6;
	pub const B_DEV_RECALIBRATE_ERROR: status_t = B_DEVICE_ERROR_BASE + 7;
	pub const B_DEV_SEEK_ERROR: status_t = B_DEVICE_ERROR_BASE + 8;
	pub const B_DEV_ID_ERROR: status_t = B_DEVICE_ERROR_BASE + 9;
	pub const B_DEV_READ_ERROR: status_t = B_DEVICE_ERROR_BASE + 10;
	pub const B_DEV_WRITE_ERROR: status_t = B_DEVICE_ERROR_BASE + 11;
	pub const B_DEV_NOT_READY: status_t = B_DEVICE_ERROR_BASE + 12;
	pub const B_DEV_MEDIA_CHANGED: status_t = B_DEVICE_ERROR_BASE + 13;
	pub const B_DEV_MEDIA_CHANGE_REQUESTED: status_t = B_DEVICE_ERROR_BASE + 14;
	pub const B_DEV_RESOURCE_CONFLICT: status_t = B_DEVICE_ERROR_BASE + 15;
	pub const B_DEV_CONFIGURATION_ERROR: status_t = B_DEVICE_ERROR_BASE + 16;
	pub const B_DEV_DISABLED_BY_USER: status_t = B_DEVICE_ERROR_BASE + 17;
	pub const B_DEV_DOOR_OPEN: status_t = B_DEVICE_ERROR_BASE + 18;

	pub const B_DEV_INVALID_PIPE: status_t = B_DEVICE_ERROR_BASE + 19;
	pub const B_DEV_CRC_ERROR: status_t = B_DEVICE_ERROR_BASE + 20;
	pub const B_DEV_STALLED: status_t = B_DEVICE_ERROR_BASE + 21;
	pub const B_DEV_BAD_PID: status_t = B_DEVICE_ERROR_BASE + 22;
	pub const B_DEV_UNEXPECTED_PID: status_t = B_DEVICE_ERROR_BASE + 23;
	pub const B_DEV_DATA_OVERRUN: status_t = B_DEVICE_ERROR_BASE + 24;
	pub const B_DEV_DATA_UNDERRUN: status_t = B_DEVICE_ERROR_BASE + 25;
	pub const B_DEV_FIFO_OVERRUN: status_t = B_DEVICE_ERROR_BASE + 26;
	pub const B_DEV_FIFO_UNDERRUN: status_t = B_DEVICE_ERROR_BASE + 27;
	pub const B_DEV_PENDING: status_t = B_DEVICE_ERROR_BASE + 28;
	pub const B_DEV_MULTIPLE_ERRORS: status_t = B_DEVICE_ERROR_BASE + 29;
	pub const B_DEV_TOO_LATE: status_t = B_DEVICE_ERROR_BASE + 30;

	// translation kit errors
	pub const B_TRANSLATION_BASE_ERROR: status_t = B_TRANSLATION_ERROR_BASE + 0;
	pub const B_NO_TRANSLATOR: status_t = B_TRANSLATION_ERROR_BASE + 1;
	pub const B_ILLEGAL_DATA: status_t = B_TRANSLATION_ERROR_BASE + 2;
}

