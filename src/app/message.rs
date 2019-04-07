//
// Copyright 2018-2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use std::char;
use std::fmt;
use std::mem::{size_of, transmute_copy, zeroed};
use std::ptr;
use std::slice::from_raw_parts;
use std::str;

use haiku_sys::{B_ANY_TYPE, B_MESSAGE_TYPE, find_thread, get_thread_info, thread_info};
use haiku_sys::errors::B_OK;

use ::app::Messenger;
use ::app::sys::*;
use ::support::{ErrorKind, Flattenable, HaikuError, Result};

/// A rustean representation of a BMessage
///
/// Like in the Haiku C++ API, the message is identified by the `what` value.
/// This one can be accessed directly.
pub struct Message {
	pub(crate) header: message_header,
	fields: Vec<field_header>,
	data: Vec<u8>
}

impl Message {
	/// Create a new message with the signature `what`
	pub fn new(what: u32) -> Self {
		Message {
			header: message_header{
				message_format: MESSAGE_FORMAT_HAIKU,
				flags: MESSAGE_FLAG_VALID,
				what: what,
				current_specifier: -1,
				message_area: -1,
				target: B_NULL_TOKEN,
				reply_target: B_NULL_TOKEN,
				reply_port: -1,
				reply_team: -1,
				data_size: 0,
				field_count: 0,
				hash_table_size: 5,
				hash_table: [-1, -1, -1, -1, -1]
			},
			fields: Vec::new(),
			data: Vec::new()
		}
	}

	pub fn what(&self) -> u32 {
		self.header.what
	}

	pub fn set_what(&mut self, what: u32) {
		self.header.what = what;
	}

	pub fn add_data<T: Flattenable<T>>(&mut self, name: &str, data: &T) {
		if self.header.message_area > 0 {
			// Todo: implement support for messages with areas
			unimplemented!()
		}
		let result = self.find_field(name, T::type_code());
		let field_index = match result {
			Some(index) => index,
			None => self.add_field(name, T::type_code(), T::is_fixed_size())
		};

		// Prepare the buffer for the copying of data
		let data_size = data.flattened_size();
		let data_size_info = if T::is_fixed_size() {
			0
		} else {
			size_of::<u32>()
		};
		let mut offset = {
			// Don't get a mutable field_header here just yet, as update_offsets
			// needs mutable references
			let field_header = self.fields.get(field_index).unwrap();
			(field_header.offset + field_header.name_length as u32 + field_header.data_size) as usize
		};
		self.data.reserve(data_size + data_size_info);
		self.update_offsets(offset, (data_size + data_size_info) as isize);

		// Actually copy the data
		// Note that there might be room for optimization by using ptr::copy
		// instead of the vector functions, especially when the field has a
		// variable size, as that now does two moves.
		if !T::is_fixed_size() {
			let data_size_vec = (data_size as u32).flatten();
			self.data.splice(offset..offset, data_size_vec.iter().cloned());
			offset += size_of::<u32>();
		}
		let data = data.flatten();
		self.data.splice(offset..offset, data.iter().cloned());

		// Update the headers
		let field_header = self.fields.get_mut(field_index).unwrap();
		field_header.count += 1;
		field_header.data_size += (data_size + data_size_info) as u32;
		self.header.data_size += (data_size + data_size_info) as u32;
	}
	
	pub fn find_data<T: Flattenable<T>>(&self, name: &str, index: usize) -> Result<T> {
		let field_index = match self.find_field(name, T::type_code()) {
			Some(index) => index,
			None => return Err(HaikuError::from(ErrorKind::NotFound)),
		};
		let field_header = &self.fields[field_index];
		if index >= field_header.count as usize {
			return Err(HaikuError::new(ErrorKind::InvalidInput, "index is out of range"));
		}
		
		if (field_header.flags & FIELD_FLAG_FIXED_SIZE) != 0 {
			let item_size: usize = (field_header.data_size / field_header.count) as usize;
			let offset: usize = (field_header.offset + field_header.name_length as u32) as usize + index * item_size;
			T::unflatten(&self.data[offset..offset+item_size])
		} else {
			let mut offset: usize = (field_header.offset + field_header.name_length as u32) as usize;
			let mut item_size: usize = 0;
			for _ in 0..index {
				// this loop will set offset to the beginning of the data that we want to read.
				// with index 0 it should at least skip the first 4 bytes (u32) that show the item size
				offset += item_size;
				item_size = u32::unflatten(&self.data[offset..offset+size_of::<u32>()]).unwrap() as usize;
				offset += size_of::<u32>();
			}
			if item_size == 0 {
				return Err(HaikuError::new(ErrorKind::InvalidData, "item size at index is garbage"));
			}
			T::unflatten(&self.data[offset..offset+item_size])
		}
	}

	pub fn replace_data<T: Flattenable<T>>(&mut self, name: &str, index: usize, data: &T) -> Result<()> {
		if self.header.message_area > 0 {
			// Todo: implement support for messages with areas
			unimplemented!()
		}

		let field_index = match self.find_field(name, T::type_code()) {
			Some(index) => index,
			None => return Err(HaikuError::from(ErrorKind::NotFound)),
		};

		let data_vec = data.flatten();

		let (offset, change) = {
			let field_header = self.fields.get_mut(field_index).unwrap();
			// Do some checks
			if index as u32 >= field_header.count {
				return Err(HaikuError::new(ErrorKind::InvalidInput, "Bad index"));
			}

			if (field_header.flags & FIELD_FLAG_FIXED_SIZE) != 0 {
				let item_size: usize = (field_header.data_size / field_header.count) as usize;
				let offset: usize = (field_header.offset + field_header.name_length as u32) as usize + index * item_size;
				self.data.splice(offset..(offset + item_size), data_vec.iter().cloned());
				(offset, 0)
			} else {
				let mut offset: usize = (field_header.offset + field_header.name_length as u32) as usize;
				let mut item_size: usize = 0;
				for _ in 0..=index {
					// this loop will set offset to the beginning of the data that we want to read.
					// with index 0 it should at least skip the first 4 bytes (u32) that show the item size
					offset += item_size;
					item_size = u32::unflatten(&self.data[offset..offset+size_of::<u32>()]).unwrap() as usize;
					offset += size_of::<u32>();
				}
				// replace the item size with the new item size
				let new_data_size = data.flattened_size();
				let data_size_vec = (new_data_size as u32).flatten();
				self.data.splice((offset - size_of::<u32>())..offset, data_size_vec.iter().cloned());
				// replace the data
				self.data.splice(offset..(offset + item_size), data_vec.iter().cloned());
				// update field header size
				field_header.data_size = field_header.data_size - item_size as u32 + new_data_size as u32;
				(offset, (offset - item_size + new_data_size) as isize)
			}
		};

		if change != 0 {
			self.update_offsets(offset, change);
		}
		// update header
		self.header.data_size = self.data.len() as u32;
		Ok(())
	}

	pub fn remove_data(&mut self, name: &str, index: usize) -> Result<()> {
		if self.header.message_area > 0 {
			// Todo: implement support for messages with areas
			unimplemented!()
		}

		let field_index = match self.find_field(name, B_ANY_TYPE) {
			Some(index) => index,
			None => return Err(HaikuError::from(ErrorKind::NotFound)),
		};

		// Optimize this check, the check is done separately to beat the borrow checker
		if index == 0 && self.fields.get(field_index).unwrap().count == 1 {
				return self.remove_field(name);
		}

		// Do some calculations with the field header
		let (offset, end) = {
			let field_header = self.fields.get_mut(field_index).unwrap();
			// Do some checks
			if index as u32 >= field_header.count {
				return Err(HaikuError::new(ErrorKind::InvalidInput, "Bad index"));
			}

			// Get the size and the offset
			if (field_header.flags & FIELD_FLAG_FIXED_SIZE) != 0 {
				let item_size: usize = (field_header.data_size / field_header.count) as usize;
				let offset: usize = (field_header.offset + field_header.name_length as u32) as usize + index * item_size;
				// Update the field header already
				field_header.data_size -= item_size as u32;
				field_header.count -= 1;
				(offset, offset + item_size)
			} else {
				let mut offset: usize = (field_header.offset + field_header.name_length as u32) as usize;
				let mut item_size: usize = 0;
				for _ in 0..=index {
					// this loop will set offset to the beginning of the data that we want to read.
					// with index 0 it should at least skip the first 4 bytes (u32) that show the item size
					offset += item_size;
					item_size = u32::unflatten(&self.data[offset..offset+size_of::<u32>()]).unwrap() as usize;
					offset += size_of::<u32>();
				}

				if item_size == 0 {
					return Err(HaikuError::new(ErrorKind::InvalidData, "item size at index is garbage"));
				}
				// Update the field header already
				field_header.data_size -= item_size as u32;
				field_header.data_size -= size_of::<u32>() as u32;
				field_header.count -= 1;
				(offset - size_of::<u32>(), offset + item_size)
			}
		};
		let empty: [u8; 0] = [];
		self.data.splice(offset..end, empty.iter().cloned());
		let change: isize = (offset as isize) - (end as isize);
		self.update_offsets(offset, change);
		self.header.data_size = ((self.header.data_size as isize) + change) as u32;
		Ok(())
	}

	pub fn remove_field(&mut self, name: &str) -> Result<()> {
		if self.header.message_area > 0 {
			// Todo: implement support for messages with areas
			unimplemented!()
		}
		let field_index = match self.find_field(name, B_ANY_TYPE) {
			Some(index) => index,
			None => return Err(HaikuError::from(ErrorKind::NotFound)),
		};

		// Get the pointers in the data stack
		let offset = {
			// Don't get a mutable field_header here just yet, as update_offsets
			// needs mutable references
			let field_header = self.fields.get(field_index).unwrap();
			field_header.offset as usize
		};
		let end = {
			// Don't get a mutable field_header here just yet, as update_offsets
			// needs mutable references
			let field_header = self.fields.get(field_index).unwrap();
			offset + (field_header.name_length as u32 + field_header.data_size) as usize
		};
		let change: isize = (offset as isize) - (end as isize);

		// Remove the data
		let empty: [u8; 0] = [];
		self.data.splice(offset..end, empty.iter().cloned());
		self.update_offsets(offset, change);

		// Update the field indexes
		// First store the index of the next field that the deleted field refers to
		let next_field = {
			let field_header = self.fields.get(field_index).unwrap();
			let next = field_header.next_field;
			if next > field_index as i32 {
				next - 1
			} else {
				next
			}
		};
		
		// Then update the hash table
		for i in 0..self.header.hash_table.len() {
			if self.header.hash_table[i] > field_index as i32 {
				self.header.hash_table[i] = self.header.hash_table[i] - 1;
			} else if self.header.hash_table[i] == field_index as i32 {
				self.header.hash_table[i] = next_field;
			}
		}
		
		// Update the indexes of each field
		for field in self.fields.iter_mut() {
			if field.next_field > field_index as i32 {
				field.next_field = field.next_field as i32 - 1;
			} else if field.next_field == field_index as i32 {
				field.next_field = next_field;
			}
		}
		
		// Remove the field
		self.fields.remove(field_index);
		
		// Update the header count
		self.header.field_count = self.header.field_count - 1;
		self.header.data_size = ((self.header.data_size as isize) + change) as u32;

		Ok(())
	}

	/// Retrieve the type, the number of items and whether or not it is fixed data
	///
	/// This method returns a tuple consisting of the type_code, the number of items
	/// and whether or not the data size is fixed, or None if there is no data.
	pub fn get_info(&self, name: &str) -> Option<(u32, usize, bool)> {
		let field_index = match self.find_field(name, B_ANY_TYPE) {
			Some(index) => index,
			None => return None
		};
		let field_header = self.fields.get(field_index).unwrap();
		Some((field_header.field_type, field_header.count as usize, (field_header.flags & FIELD_FLAG_FIXED_SIZE) != 0))
	}

	/// Check if the message has data associated with it
	pub fn is_empty(&self) -> bool {
		self.fields.len() == 0
	}

	/// Check if the message is a system message
	///
	/// System messages have a what code that is built up from the '_'
	/// (underscore) character, and three capital letters. Example: `_BED`
	/// Because of this fact it is advised not to give your message codes
	/// this structure.
	pub fn is_system(&self) -> bool {
		let a: char = char::from_u32((self.what() >> 24) & 0xff).unwrap_or('x');
		let b: char = char::from_u32((self.what() >> 16) & 0xff).unwrap_or('x');
		let c: char = char::from_u32((self.what() >> 8) & 0xff).unwrap_or('x');
		let d: char = char::from_u32(self.what() & 0xff).unwrap_or('x');
		if a == '_' && b.is_ascii_uppercase() && c.is_ascii_uppercase() &&
			d.is_ascii_uppercase() {
				true
		} else {
			false
		}
	}

	/// Check if the message is a reply message
	pub fn is_reply(&self) -> bool {
		(self.header.flags & MESSAGE_FLAG_IS_REPLY) != 0
	}

	/// Check if this message was delivered through a messenger
	pub fn was_delivered(&self) -> bool {
		(self.header.flags & MESSAGE_FLAG_WAS_DELIVERED) != 0
	}

	/// Check if the source is waiting for a reply
	pub fn is_source_waiting(&self) -> bool {
		(self.header.flags & MESSAGE_FLAG_REPLY_REQUIRED) != 0
			&& (self.header.flags & MESSAGE_FLAG_REPLY_DONE) != 0
	}

	/// Check if the source is another application than the current
	pub fn is_source_remote(&self) -> bool {
		// Compare the team id to the message team id. 
		// The following code to get the team id could be extracted and made reusable
		let team = unsafe {
			let mut info: thread_info = zeroed();
			let id = find_thread(ptr::null());
			println!("id: {}", id);
			let retval =  get_thread_info(id, &mut info);
			println!("retval: {}", retval);
			if get_thread_info(id, &mut info) != B_OK {
				panic!("Cannot get the thread_info for the current thread")
			}
			info.team
		};
		println!("team: {}, reply_team: {}", team, self.header.reply_team);
		(self.header.flags & MESSAGE_FLAG_WAS_DELIVERED) != 0
			&& self.header.reply_team != team
	}
	
	/// Get a Messenger to the sender of this message
	pub fn get_return_address(&self) -> Option<Messenger> {
		if (self.header.flags & MESSAGE_FLAG_WAS_DELIVERED) == 0 {
			return None;
		}
		
		Messenger::from_port_id(self.header.reply_port)
	}

	fn hash_name(&self, name: &str) -> u32 {
		let mut result: u32 = 0;
		for byte in name.bytes() {
			result = (result << 7) ^ (result >> 24);
			result ^= byte as u32;
		}
		
		result ^= result << 12;
		result
	}

	fn find_field(&self, name: &str, type_code: u32) -> Option<usize> {
		if name.len() == 0 {
			return None
		}
		
		if self.header.field_count == 0 {
			return None
		}
		
		let hash = self.hash_name(name) % self.header.hash_table_size;
		let mut next_index = self.header.hash_table[hash as usize];
		while next_index >= 0 {
			let field = &self.fields[next_index as usize];
			let start = field.offset as usize;
			let end = (field.offset + field.name_length as u32 - 1) as usize; // do not add trailing \0 to range
			if *name.as_bytes() == self.data[start..end] {
				if field.field_type == type_code || type_code == B_ANY_TYPE {
					return Some(next_index as usize);
				} else {
					return None
				}
			}
			
			next_index = field.next_field;
		}
		None
	}
	
	fn add_field(&mut self, name: &str, type_code: u32, is_fixed_size: bool) -> usize {
		// BMessage has an optimization where some headers are pre-allocated
		// to avoid reallocating the header array. We should implement this,
		// TODO: Vec::with_capacity can help with implementing this
		let mut flags: u16 = FIELD_FLAG_VALID;
		if is_fixed_size {
			flags |= FIELD_FLAG_FIXED_SIZE;
		}
		
		let hash: u32 = self.hash_name(name) % self.header.hash_table_size;
		let mut current_index: i32 = self.header.hash_table[hash as usize];
		if current_index >= 0 {
			{
				let mut next_field: &field_header = &self.fields[current_index as usize];
				while next_field.next_field >= 0 {
					current_index = next_field.next_field;
					next_field = &self.fields[current_index as usize];
				}
			}
			self.fields.get_mut(current_index as usize).unwrap().next_field = self.header.field_count as i32;
		} else {
			self.header.hash_table[hash as usize] = self.header.field_count as i32;
		}
		
		self.fields.push(field_header {
			flags: flags,
			name_length: name.len() as u16 + 1,
			field_type: type_code,
			count: 0,
			data_size: 0,
			offset: self.header.data_size,
			next_field: -1
		});

		self.header.field_count += 1;
		self.header.data_size += (name.len() as u32) + 1;

		// Store name to the vector
		let data_size = (name.len() as u32) + 1;
		self.data.reserve(data_size as usize);
		for byte in name.as_bytes() {
			self.data.push(*byte);
		}
		self.data.push('\0' as u8);
		
		return (self.header.field_count - 1) as usize;
	}

	fn update_offsets(&mut self, offset: usize, change: isize) {
		if offset < self.data.len() {
			for field in self.fields.iter_mut() {
				if field.offset as usize >= offset {
					field.offset = ((field.offset as isize) + change) as u32;
				}
			}
		}
	}
}

impl Flattenable<Message> for Message {
	fn type_code() -> u32 {
		B_MESSAGE_TYPE
	}
	
	fn flattened_size(&self) -> usize {
		return size_of::<message_header>() + size_of::<field_header>() * self.fields.len() + self.data.len();
	}
	
	fn is_fixed_size() -> bool {
		false
	}
	
	fn flatten(&self) -> Vec<u8> {
		let mut vec: Vec<u8> = vec![0;self.flattened_size()];
		// Copy message header
		{
			let (message_header_slice, _) = vec.as_mut_slice().split_at_mut(size_of::<message_header>());
			let message_header_bytes: &[u8] = unsafe { 
				from_raw_parts((&self.header as *const message_header) as *const u8, size_of::<message_header>())
			};
			message_header_slice.clone_from_slice(message_header_bytes);
		}
		// Copy field headers and data
		if self.fields.len() > 0 {
			{
				let (_, field_header_slice) = vec.as_mut_slice().split_at_mut(size_of::<message_header>());
				let field_header_bytes: &[u8] = unsafe { 
					from_raw_parts((self.fields.as_slice() as *const [field_header]) as *const u8, size_of::<field_header>())
				};
				unsafe {
					ptr::copy_nonoverlapping(field_header_bytes.as_ptr(), field_header_slice.as_mut_ptr(), size_of::<field_header>() * self.fields.len());
				}
			}
			{
				// Copy data
				let(_, data_slice) = vec.as_mut_slice().split_at_mut(size_of::<message_header>() + size_of::<field_header>() * self.fields.len());
				unsafe {
					ptr::copy_nonoverlapping(self.data.as_ptr(), data_slice.as_mut_ptr(), self.data.len());
				}
			}
		}
		
		vec
	}
	
	fn unflatten(buffer: &[u8]) -> Result<Message> {
		// minimum size is at least the header
		if buffer.len() < size_of::<message_header>() {
			return Err(HaikuError::new(ErrorKind::InvalidData, "buffer size is shorter than a message"));
		}
		// check the first 4 bytes and compare the message constant
		if buffer[0] != 'H' as u8 || buffer[1] != 'M' as u8 || buffer[2] != 'F' as u8 || buffer[3] != '1' as u8 {
			return Err(HaikuError::new(ErrorKind::InvalidData, "buffer does not contain a valid haiku message"));
		}

		let data_ptr: *const u8 = buffer.as_ptr();
		let header_ptr: *const message_header = data_ptr as *const _;
		let header_ref: &message_header = unsafe { &*header_ptr };

		let mut msg = Message{
			header: header_ref.clone(),
			fields: Vec::new(),
			data: Vec::new()
		};

		let total_size = size_of::<message_header>() + size_of::<field_header>() * msg.header.field_count as usize + msg.header.data_size as usize;
		
		if total_size != buffer.len() {
			return Err(HaikuError::new(ErrorKind::InvalidData, "buffer is smaller than the advertised message size"));
		}

		let mut offset = size_of::<message_header>();
		for _ in 0..msg.header.field_count {
			let (_, field_header_slice) = buffer.split_at(offset);
			let field_header_ptr: *const field_header = field_header_slice.as_ptr() as *const _;
			let field_header_ref: &field_header = unsafe {&*field_header_ptr };
			msg.fields.push(field_header_ref.clone());
			offset += size_of::<field_header>();
		}

		let (_, data_part_slice) = buffer.split_at(offset);
		msg.data.extend_from_slice(data_part_slice);

		Ok(msg)
	}
}

impl fmt::Debug for Message {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// TODO: make this mirror BMessage::PrintToStream()
		let chars = unsafe { transmute_copy::<u32, [u8; 4]>(&self.what()) };
		let mut print_chars = true;
		for ch in chars.iter() {
			if !(*ch as char).is_ascii_graphic() {
				print_chars = false;
				break;
			}
		}
		
		let result = if print_chars {
			write!(f, "BMessage: ({:?})", (chars[3] as char, chars[2] as char, chars[1] as char, chars[0] as char))
		} else {
			write!(f, "BMessage: ({})", self.what())
		};
		
		if self.fields.len() > 0 {
			write!(f, "\n{{\n").ok();
			for field in self.fields.iter() {
				let name_slice = &self.data[(field.offset as usize)..(field.offset + field.name_length as u32) as usize];
				write!(f, "\t{}\n", str::from_utf8(name_slice).unwrap()).ok();
			}
			write!(f, "}}")
		} else {
			result
		}
	}
}

#[test]
fn test_message_add_and_remove() {
	let constant: u32 = haiku_constant!('a', 'b', 'c', 'd');
	let mut message = Message::new(constant);
	message.add_data("parameter1", &(15 as i8));
	message.add_data("parameter2", &String::from("value1"));
	message.add_data("parameter1", &(51 as i8));
	message.add_data("parameter2", &String::from("value2"));

	let flattened_message = message.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 100, 99, 98, 97, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 46, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 3, 0, 11, 0, 69, 84, 89, 66, 2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 11, 0, 82, 84, 83, 67, 2, 0, 0, 0, 22, 0, 0, 0, 13, 0, 0, 0, 255, 255, 255, 255, 112, 97, 114, 97, 109, 101, 116, 101, 114, 49, 0, 15, 51, 112, 97, 114, 97, 109, 101, 116, 101, 114, 50, 0, 7, 0, 0, 0, 118, 97, 108, 117, 101, 49, 0, 7, 0, 0, 0, 118, 97, 108, 117, 101, 50, 0);
	assert_eq!(flattened_message, comparison);

	message.add_data("parameter3", &(40 as i8));
	assert!(message.remove_field("parameter1").is_ok());
	assert!(message.remove_field("parameter3").is_ok());
	let flattened_message = message.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 100, 99, 98, 97, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 33, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 1, 0, 11, 0, 82, 84, 83, 67, 2, 0, 0, 0, 22, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 112, 97, 114, 97, 109, 101, 116, 101, 114, 50, 0, 7, 0, 0, 0, 118, 97, 108, 117, 101, 49, 0, 7, 0, 0, 0, 118, 97, 108, 117, 101, 50, 0);
	assert_eq!(flattened_message, comparison);
	
	message.add_data("parameter2", &String::from("value n+1 much longer"));
	assert!(message.remove_data("parameter2", 1).is_ok());
	let flattened_message = message.flatten();
	let comparison: Vec<u8> =  vec!(72, 77, 70, 49, 100, 99, 98, 97, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 48, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 1, 0, 11, 0, 82, 84, 83, 67, 2, 0, 0, 0, 37, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 112, 97, 114, 97, 109, 101, 116, 101, 114, 50, 0, 7, 0, 0, 0, 118, 97, 108, 117, 101, 49, 0, 22, 0, 0, 0, 118, 97, 108, 117, 101, 32, 110, 43, 49, 32, 109, 117, 99, 104, 32, 108, 111, 110, 103, 101, 114, 0);
	assert_eq!(flattened_message, comparison);
}

#[test]
fn test_message_replace() {
	let constant: u32 = haiku_constant!('q', 'w', 'e', 'r');
	let mut message = Message::new(constant);
	message.add_data("parameter1", &(159498393898 as i64));
	message.add_data("parameter1", &(940030747479 as i64));
	message.add_data("parameter1", &(573678299939 as i64));
	message.add_data("parameter2", &String::from("str1"));
	message.add_data("parameter2", &String::from("string2"));
	message.add_data("parameter2", &String::from("string number 3"));
	let flattened_message = message.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 114, 101, 119, 113, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 87, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 3, 0, 11, 0, 71, 78, 76, 76, 3, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 11, 0, 82, 84, 83, 67, 3, 0, 0, 0, 41, 0, 0, 0, 35, 0, 0, 0, 255, 255, 255, 255, 112, 97, 114, 97, 109, 101, 116, 101, 114, 49, 0, 42, 89, 216, 34, 37, 0, 0, 0, 87, 227, 50, 222, 218, 0, 0, 0, 35, 43, 228, 145, 133, 0, 0, 0, 112, 97, 114, 97, 109, 101, 116, 101, 114, 50, 0, 5, 0, 0, 0, 115, 116, 114, 49, 0, 8, 0, 0, 0, 115, 116, 114, 105, 110, 103, 50, 0, 16, 0, 0, 0, 115, 116, 114, 105, 110, 103, 32, 110, 117, 109, 98, 101, 114, 32, 51, 0);
	assert_eq!(flattened_message, comparison);

	// Replace data with fixed size
	assert!(message.replace_data("parameter1", 1, &(-4939497933 as i64)).is_ok());
	let flattened_message = message.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 114, 101, 119, 113, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 87, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 3, 0, 11, 0, 71, 78, 76, 76, 3, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 11, 0, 82, 84, 83, 67, 3, 0, 0, 0, 41, 0, 0, 0, 35, 0, 0, 0, 255, 255, 255, 255, 112, 97, 114, 97, 109, 101, 116, 101, 114, 49, 0, 42, 89, 216, 34, 37, 0, 0, 0, 51, 62, 149, 217, 254, 255, 255, 255, 35, 43, 228, 145, 133, 0, 0, 0, 112, 97, 114, 97, 109, 101, 116, 101, 114, 50, 0, 5, 0, 0, 0, 115, 116, 114, 49, 0, 8, 0, 0, 0, 115, 116, 114, 105, 110, 103, 50, 0, 16, 0, 0, 0, 115, 116, 114, 105, 110, 103, 32, 110, 117, 109, 98, 101, 114, 32, 51, 0);
	assert_eq!(flattened_message, comparison);

	// Replace data with variable size
	assert!(message.replace_data("parameter2", 1, &String::from("longer string 2")).is_ok());
	let flattened_message = message.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 114, 101, 119, 113, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 95, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 3, 0, 11, 0, 71, 78, 76, 76, 3, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 11, 0, 82, 84, 83, 67, 3, 0, 0, 0, 49, 0, 0, 0, 35, 0, 0, 0, 255, 255, 255, 255, 112, 97, 114, 97, 109, 101, 116, 101, 114, 49, 0, 42, 89, 216, 34, 37, 0, 0, 0, 51, 62, 149, 217, 254, 255, 255, 255, 35, 43, 228, 145, 133, 0, 0, 0, 112, 97, 114, 97, 109, 101, 116, 101, 114, 50, 0, 5, 0, 0, 0, 115, 116, 114, 49, 0, 16, 0, 0, 0, 108, 111, 110, 103, 101, 114, 32, 115, 116, 114, 105, 110, 103, 32, 50, 0, 16, 0, 0, 0, 115, 116, 114, 105, 110, 103, 32, 110, 117, 109, 98, 101, 114, 32, 51, 0);
	assert_eq!(flattened_message, comparison);
}

#[test]
fn test_message_flattening() {
	let constant: u32 = haiku_constant!('a', 'b', 'c', 'd');
	let basic_message = Message::new(constant);
	let flattened_message = basic_message.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 100, 99, 98, 97, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255);
	assert_eq!(flattened_message, comparison);
	assert!(!basic_message.is_reply());
	assert!(!basic_message.is_source_remote());

	// Second message
	let constant: u32 = haiku_constant!('e', 'f', 'g', 'h');
	let mut message_with_data = Message::new(constant);
	assert!(message_with_data.is_empty());
	message_with_data.add_data("UInt8", &('a' as u8));
	message_with_data.add_data("UInt16", &(1234 as u16));
	assert!(!message_with_data.is_empty());
	let flattened_message = message_with_data.flatten();
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 104, 103, 102, 101, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 16, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 3, 0, 6, 0, 84, 89, 66, 85, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 3, 0, 7, 0, 84, 72, 83, 85, 1, 0, 0, 0, 2, 0, 0, 0, 7, 0, 0, 0, 255, 255, 255, 255, 85, 73, 110, 116, 56, 0, 97, 85, 73, 110, 116, 49, 54, 0, 210, 4);
	assert_eq!(flattened_message, comparison);
	
	// Third message
	let constant: u32 = haiku_constant!('l', 'n', 'd', 'a');
	let mut app_data_message = Message::new(constant);
	app_data_message.add_data("name", &String::from("application/x-vnd.haiku-registrar"));
	app_data_message.add_data("user", &(0));
	let comparison: Vec<u8> = vec!(72, 77, 70, 49, 97, 100, 110, 108, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 52, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 1, 0, 5, 0, 82, 84, 83, 67, 1, 0, 0, 0, 38, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 3, 0, 5, 0, 71, 78, 79, 76, 1, 0, 0, 0, 4, 0, 0, 0, 43, 0, 0, 0, 255, 255, 255, 255, 110, 97, 109, 101, 0, 34, 0, 0, 0, 97, 112, 112, 108, 105, 99, 97, 116, 105, 111, 110, 47, 120, 45, 118, 110, 100, 46, 104, 97, 105, 107, 117, 45, 114, 101, 103, 105, 115, 116, 114, 97, 114, 0, 117, 115, 101, 114, 0, 0, 0, 0, 0);
	let flattened_message = app_data_message.flatten();
	assert_eq!(flattened_message, comparison);
}

#[test]
fn test_system_message() {
	let system_constant: u32 = haiku_constant!('_','A','B','C');
	let system_message = Message::new(system_constant);
	assert!(system_message.is_system());

	let other_constant: u32 = haiku_constant!('x','A','B','C');
	let other_message = Message::new(other_constant);
	assert!(!other_message.is_system());
}
