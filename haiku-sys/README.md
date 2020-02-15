# Low level Rust FFI crate of Haiku system interfaces

This crate offers access to the low level Haiku system functions, data
structures and constants, that are not available in `libc`.

For a higher-level Rust crate for Haiku, look for the `haiku` crate.

## Using the crate

This crate is published on crates.io and can be used by adding it as a
dependency in your `Cargo.toml` file. 

## What is implemented

Currently the following kernel interfaces are implemented:

* Areas
* Ports
* Thread (partial, info only)
* Attribute functions
* Image (partial, info only)

## What is still to be done

* Teams
* Time & Alarm
* System Information
* FS Index
* FS Info
* FS Query
* FS Volume

## What probably will never be done

* Native semaphores
* Native threads
* Native signals
* Image loading
