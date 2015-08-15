#! /bin/sh
mkdir -p objects

# build the crate
rustc --crate-type lib -o objects/libhaiku.so src/lib.rs
rustc --crate-type lib --test -o objects/test src/lib.rs
