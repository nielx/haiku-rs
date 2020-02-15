# Rust implementation of the Haiku API

The goal of this crate is to provide a Rust implementation of the
Haiku API. These functions can mostly be classified as low-level functions,
which can be (mostly) found in the kernel kit. Where it makes sense, a higher
level API is implemented which uses more advanced rust language constructs.

## Building

Using `cargo build` should do the trick!

## Using the crate

This crate is published on crates.io and can be used by adding it as a
dependency in your `Cargo.toml` file. 

## What is implemented

Currently the following kernel interfaces are implemented:

* Application Kit
  - Initial implementation of Handlers, Loopers, Messengers and Messages
  - Access to the Registrar (limited for now)
  - Notification system
* Kernel Kit
  - High level interface to Ports and Teams
* Storage Kit
  - File Attributes
  - Basic MimeType
* Support Kit
  - Haiku specific Error object
  - Flattenable interface

The implementation of the messaging system currently is at a high enough level
that it may be used to communicate with other Haiku applications and system
services.

## Further information

This crate is developed on [github](https://github.com/nielx/haiku-rs)
