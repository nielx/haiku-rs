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

* Attribute functions
* Debugger (the call, not the interface)
* File attributes
* Messaging
* Ports

## What is still to be done

* Areas
* Teams
* Time & Alarm
* System Information
* FS Index
* FS Info
* FS Query
* FS Volume
* More access to the Registrar

## What probably will never be done

* Native semaphores
* Native threads
* Native signals
* Image loading
