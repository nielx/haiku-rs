# Rust bindings for the Haiku API

The goal of this package is to provide access to the C-style functions of the
Haiku API. These functions can mostly be classified as low-level functions,
which can be (mostly) found in the kernel kit. Where it makes sense, a higher
level API is implemented which uses more advanced rust language constructs.

## Building

Since `cargo` is not available for Haiku yet, building is done using `rustc`
directly. All of this is automated in the `build.sh` script. Invoke it to
generate the library, the test app, and the samples in the `objects` directory.

## Using the library

Right now the library is so small that it makes little sense to dynamically
link it in your own projects. Therefore statically linking against
`libhaiku.rlib` is fine. 

In order to link against this library, you need to specify the crate in your
source file:

    ```
    extern crate haiku;
    ```

Using the `rustc -L <path_to_libhaiku> <main source file>` you can make the
compiler find the crate. 

## What is implemented

Currently the following kernel interfaces are implemented:

* Attribute functions
* Debugger (the call, not the interface)
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

## What probably will never be done

* Native semaphores
* Native threads
* Native signals
* Image loading
