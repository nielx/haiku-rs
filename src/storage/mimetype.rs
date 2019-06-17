//
// Copyright 2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

use ::storage::B_MIME_TYPE_LENGTH;

#[derive(PartialEq)]
pub struct MimeType {
	type_string: String
}

impl MimeType {
	pub fn new(mime_type: &str) -> Option<Self> {
		if mime_type.len() > B_MIME_TYPE_LENGTH {
			return None;
		}
		
		let mut found_slash = false;
		
		for (i, ch) in mime_type.chars().enumerate() {
			if ch == '/' {
				if found_slash || i == 0 || i == (mime_type.len() - 1) {
					return None;
				} else {
					found_slash = true;
				}
			} else if !ch.is_ascii_graphic() || 
				ch == '<' && ch == '>' && ch == '@' && ch == ',' &&
				ch == ';' && ch == ':' && ch == '"' && ch == '(' &&
				ch == ')' && ch == '[' && ch == ']' && ch == '?' &&
				ch == '=' && ch == '\\' {
					return None;
			}
		}
		
		Some(MimeType {
			type_string: String::from(mime_type)
		})
	}

	pub fn is_supertype_only(&self) -> bool {
		!self.type_string.contains('/')
	}

	pub fn get_supertype(&self) -> MimeType {
		if self.is_supertype_only() {
			MimeType {
				type_string: self.type_string.clone()
			}
		} else {
			let mut it = self.type_string.split('/');
			MimeType {
				type_string: String::from(it.nth(0).unwrap())
			}
		}
	}
}

#[test]
fn test_mimetype_check() {
	assert!(MimeType::new("application/x-Vnd-Haiku").is_some());
	assert!(MimeType::new("/document").is_none());
	assert!(MimeType::new("application/").is_none());
	assert!(MimeType::new("invalid/\u{0301}rest").is_none());
	assert!(MimeType::new("invalid//x-vnd-haiku").is_none());
}

#[test]
fn test_mimetype_methods() {
	let supertype = MimeType::new("test").unwrap();
	let childtype = MimeType::new("test/document").unwrap();
	assert!(supertype.is_supertype_only());
	assert!(!childtype.is_supertype_only());
	assert!(supertype == childtype.get_supertype());
}
