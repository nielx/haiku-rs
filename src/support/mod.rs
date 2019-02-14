//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//


mod errors;
mod flattenable;

pub use self::errors::{ErrorKind, HaikuError, Result};
pub use self::flattenable::Flattenable;
