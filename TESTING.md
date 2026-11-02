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

`test.sh` checks the layout (`cargo fmt`) and Clippy's lints, runs the tests with every feature on and with none, and
builds each format alone and for the browser's `wasm32-unknown-unknown`. CI (`.github/workflows/ci.yml`, GitHub Actions)
runs it and the viewer's `test.sh` below, and leaves the oracle to be run by hand: ufbx's C allocates what a damaged
file declares, and on a machine that refuses it, as GitHub's runners do, C answers "Out of memory" where this crate
reports the file's own error, which the comparison counts as a failure. `vm.overcommit_memory=1` lets it pass.

This repository holds no 3D file. The checks above take ufbx's from its public repository, cloned beside this one as
`../ufbx`. `cargo test` needs no clone: the crate's tests write their files in code, and the three that read ufbx's are
ignored until asked for, with the long test of decimals:

    UFBX_DATA=../ufbx/data cargo test --release --lib -- --ignored

`oracle/ufbx-compat` runs ufbx-rust's own tests on `load3d::compat`, unchanged and not copied, from its clone beside
this one, but for `blender_default_custom_stream_skip`, which asks a stream to skip. The oracle's section `compat`
compares the layer with ufbx-rust itself, field by field, over ufbx's files.

`examples/viewer/test.sh` builds the viewer for the browser and opens it in a headless Chromium in Docker on a file of
each format, which the test writes itself; the page it builds opens from any static server:

    examples/viewer/test.sh && python3 -m http.server -d examples/viewer/target/site

`./test-all.sh` runs everything: `test.sh` and `examples/viewer/test.sh`, then the crate's ignored tests, the oracle's
checks above and ufbx-rust's tests. `UFBX_PATH` and `UFBX_RUST_PATH` name the clones, `../ufbx` and `../ufbx-rust`
unless set; a missing one is cloned at the release the oracle compares with.
