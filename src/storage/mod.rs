//
// Copyright 2015, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

///! Tools to manipulate the file system and the Haiku specific extentions to
///! it

mod attributes;

pub use self::attributes::{AttributeDescriptor, AttributeIterator, AttributeExt};
