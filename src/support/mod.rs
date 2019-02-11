//
// Copyright 2018, Niels Sascha Reedijk <niels.reedijk@gmail.com>
// All rights reserved. Distributed under the terms of the MIT License.
//


mod errors;
pub mod flattenable;

pub use self::errors::Result as Result;
pub use self::errors::HaikuError as HaikuError;
pub use self::errors::ErrorKind as ErrorKind;
