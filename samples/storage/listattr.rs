extern crate haiku;

use std::env::args;
use std::path::Path;

use haiku::kernel::type_constants::*;
use haiku::storage::attributes::{AttributeDescriptor, AttributeIterator, AttributeExt};

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
		println!("{: >12} {: >10}  {: <36} {: <21}", "Type", "Size", "Name", "Contents");
		println!("{}", (0..73).map(|_| "-").collect::<String>());
		for x in attribute_iterator {
			if let Ok(attribute) = x {
				println!("{: >12} {: >10}  {: <36}", get_type(attribute.raw_attribute_type), attribute.size, attribute.name);
			} else {
				println!("Breaking loop because of error");
				break;
			}
		}
	}
}
