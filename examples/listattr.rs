extern crate haiku;
extern crate haiku_sys;

use std::env::args;
use std::fmt::Write;
use std::path::Path;

use haiku_sys::*;
use haiku::storage::{AttributeDescriptor, AttributeExt};

fn get_type(type_code: u32) -> String {
	match type_code {
		B_MIME_STRING_TYPE => "MIME String".to_string(),
		B_STRING_TYPE => "Text".to_string(),
		B_BOOL_TYPE => "Boolean".to_string(),
		B_DOUBLE_TYPE => "Double".to_string(),
		B_FLOAT_TYPE => "Float".to_string(),
		B_INT8_TYPE => "Int-8".to_string(),
		B_INT16_TYPE => "Int-16".to_string(),
		B_INT32_TYPE => "Int-32".to_string(),
		B_INT64_TYPE => "Int-64".to_string(),
		B_UINT8_TYPE => "Uint-8".to_string(),
		B_UINT16_TYPE => "Uint-16".to_string(),
		B_UINT32_TYPE => "Uint-32".to_string(),
		B_UINT64_TYPE => "Uint-64".to_string(),
		_ => "Other".to_string(), // TODO: convert into character string
	}
}

fn print_attribute_contents(path: &Path, attribute: &AttributeDescriptor) -> String {
	let mut output = String::new();
	match attribute.raw_attribute_type {
		B_INT8_TYPE => {
			let value = path.read_attribute::<i8>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_UINT8_TYPE => {
			let value = path.read_attribute::<u8>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_INT16_TYPE => {
			let value = path.read_attribute::<i16>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_UINT16_TYPE => {
			let value = path.read_attribute::<u16>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_INT32_TYPE => {
			let value = path.read_attribute::<i32>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_UINT32_TYPE => {
			let value = path.read_attribute::<u32>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_INT64_TYPE => {
			let value = path.read_attribute::<i64>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_UINT64_TYPE => {
			let value = path.read_attribute::<u64>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_FLOAT_TYPE => {
			let value = path.read_attribute::<f32>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_DOUBLE_TYPE => {
			let value = path.read_attribute::<f64>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_BOOL_TYPE => {
			let value = path.read_attribute::<bool>(attribute).unwrap();
			write!(&mut output, "{}", value).unwrap();
		},
		B_STRING_TYPE => {
			let value = path.read_attribute::<String>(attribute).unwrap();
			write!(&mut output, "{}",  value).unwrap();
		},
		_ => {
			write!(&mut output, "{}", dump_raw_data(&path, &attribute)).unwrap()
		}
	}
	output
}

fn dump_raw_data(path: &Path, attribute: &AttributeDescriptor) -> String {
	const CHUNK_SIZE: u32 = 16;
	const PRINT_LIMIT: i64 = 256;
	let mut dump_position: usize = 0;
	let mut output = String::from("\n");

	let buffer = path.read_attribute_raw(&attribute.name, attribute.raw_attribute_type, 0, PRINT_LIMIT).unwrap();

	while dump_position < buffer.len() {
		write!(&mut output, "\t{:>4}: ", dump_position).unwrap();
		for i in 0..CHUNK_SIZE as usize {
			if dump_position + i < buffer.len() {
				write!(&mut output, "{:2x} ", buffer[dump_position + i]).unwrap();
			}else {
				write!(&mut output, "   ").unwrap();
			}
		}

		// Print the byte in the form of a printable character
		output.push(' ');
		for _ in 0..CHUNK_SIZE {
			if dump_position < buffer.len() {
				let c = buffer[dump_position] as char;
				if c.is_ascii_graphic() {
					write!(&mut output, "{}", c).unwrap();
				} else {
					output.push('.');
				}
			} else {
				output.push(' ');
			}
			dump_position += 1;
		}
		output.push('\n');
	}
	output
}

fn main() {
	let mut printcontents = false;
	let mut args: Vec<_> = args().collect();

	if args.contains(&"-h".to_string()) || args.contains(&"--help".to_string())
		|| args.len() == 1 {
		println!("usage: listattr [-l|--long] 'filename' ['filename' ...]");
		println!("   -l, --long  Shows the attribute contents as well.");
		return;
	}

	if args.len() > 2 && (args[1] == "-l" || args[1] == "--long") {
		printcontents = true; 
		args.remove(1);
	}

	for arg in args {
		let path = Path::new(&arg);
		let attribute_iterator = path.iter_attributes().unwrap();
		
		println!("File: {}", arg);
		const TYPE_WIDTH: usize = 12;
		const SIZE_WIDTH: usize =10;
		const NAME_WIDTH: usize = 36;
		const CONTENTS_WIDTH: usize = 21;
		
		let contents_header = if printcontents {
			"Contents"
		} else {
			""
		};
		let header_length = if printcontents {
			TYPE_WIDTH + SIZE_WIDTH + NAME_WIDTH + CONTENTS_WIDTH
		} else {
			TYPE_WIDTH + SIZE_WIDTH + NAME_WIDTH
		};

		println!("{0: >1$} {2: >3$}  {4: <5$} {6}", "Type", TYPE_WIDTH, "Size", SIZE_WIDTH, "Name", NAME_WIDTH, contents_header);
		println!("{}", (0..header_length).map(|_| "-").collect::<String>());
		for x in attribute_iterator {
			if let Ok(attribute) = x {
				let contents = if printcontents {
					print_attribute_contents(&path, &attribute) 
				} else {
					String::new()
				};
				println!("{0: >1$} {2: >3$}  {4: <5$} {6}", get_type(attribute.raw_attribute_type), TYPE_WIDTH, attribute.size, SIZE_WIDTH, attribute.name, NAME_WIDTH, &contents);
			} else {
				println!("Breaking loop because of error");
				break;
			}
		}
	}
}
