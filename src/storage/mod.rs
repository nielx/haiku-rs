//
// Copyright 2015-2019, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

//! Tools to manipulate the file system and the Haiku specific extentions to
//! it

use libc::{FILENAME_MAX, PATH_MAX};

mod attributes;
mod mimetype;
pub(crate) mod sys;

pub use self::attributes::{AttributeDescriptor, AttributeExt, AttributeIterator};
pub use self::mimetype::MimeType;

// Kit constants
/// Maximum length for the name of a device
pub const B_DEV_NAME_LENGTH: usize = 128;
/// Maximum length for the name of a file
pub const B_FILE_NAME_LENGTH: usize = FILENAME_MAX as usize;
/// Maximum length for a full path
pub const B_PATH_NAME_LENGTH: usize = PATH_MAX as usize;
/// Maximum length for the name of an attribute
pub const B_ATTR_NAME_LENGTH: usize = FILENAME_MAX as usize - 1;
/// Maximum length of a mime type
pub const B_MIME_TYPE_LENGTH: usize = B_ATTR_NAME_LENGTH as usize - 15;
/// Maximum number of connected symlinks that will be followed, before an
/// operation fails.
pub const B_MAX_SYMLINKS: usize = 16; // This does not seem to be exposed in libc
