//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//

//! The support kit provides a few fundamentals that are used in Haiku applications

mod errors;
mod flattenable;

pub use self::errors::{ErrorKind, HaikuError, Result};
pub use self::flattenable::Flattenable;
