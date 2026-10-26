# How load3d is checked

`oracle/` builds ufbx itself through the [`ufbx`](https://crates.io/crates/ufbx) crate and compares, field by field
and bit for bit, every section of this crate with it over ufbx's own test files, over damaged copies of them, and
through files, streams, small read buffers, progress callbacks and cancels:

    git clone --depth 1 --branch "$(cd oracle && cargo run --release -q -- ufbx-version)" https://github.com/ufbx/ufbx ../ufbx
    cd oracle && cargo run --release -- all --corpus ../../ufbx/data

`oracle/tests/` is ufbx's own test suite, `test/` of the same tag, on this crate: its runner, its check of a scene, its
comparison with the `.obj` files the modelling programs exported, and its 658 test cases under their names, every file
test loaded in every version and format ufbx has of its file, as ufbx's CI runs them. What has no counterpart in Rust is
left out where it stands, with why: ufbx's thread pool, its allocators and their limits, reference counts, NULL
arguments, bytes a `String` cannot hold.

    cd oracle && UFBX_DATA=../../ufbx/data cargo test --release --test '*'

`UFBXT_FUZZ=1` adds ufbx's `--fuzz`: every truncation of each file, every cancel, every byte patched, every read
buffer size.

The browser build needs its target, once:

    rustup target add wasm32-unknown-unknown

`test.sh` runs the tests with every feature on and with none, and builds each format alone and for the
browser's `wasm32-unknown-unknown`.

This repository holds no 3D file. The checks above take ufbx's from its public repository, cloned beside this one as
`../ufbx`.
