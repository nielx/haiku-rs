#! /bin/sh
mkdir -p objects

# build the crate
rustc --out-dir objects src/lib.rs
rustc -g --crate-type lib -C prefer-dynamic --test -o objects/test src/lib.rs

# build the docs
rustdoc src/lib.rs

# build the samples
rustc --out-dir objects -L objects samples/storage/listattr.rs
