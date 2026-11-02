#!/bin/sh
# UFBX_PATH and UFBX_RUST_PATH name the clones, ../ufbx and ../ufbx-rust beside this repository unless set; a missing one
# is cloned at the release the oracle compares with.
set -eux

root=$(cd "$(dirname "$0")" && pwd)
ufbx=${UFBX_PATH:-$root/../ufbx}
if [ ! -d "$ufbx/data" ]; then
    version=$(cd "$root/oracle" && cargo run --release --locked -q -- ufbx-version)
    git clone --depth 1 --branch "$version" https://github.com/ufbx/ufbx "$ufbx"
fi
UFBX_DATA=$(cd "$ufbx/data" && pwd)
export UFBX_DATA

rust=${UFBX_RUST_PATH:-$root/../ufbx-rust}
if [ ! -d "$rust/tests" ]; then
    git clone --quiet https://github.com/ufbx/ufbx-rust "$rust"
    git -C "$rust" checkout --quiet "$(cat "$root/oracle/ufbx-compat/ufbx-rust.commit")"
fi
rust=$(cd "$rust" && pwd)

cd "$root"
./test.sh
examples/viewer/test.sh
cargo test --release --locked --lib -- --ignored

cd oracle
cargo test --release --locked --lib --bins
cargo run --release --locked -- all --corpus "$UFBX_DATA"
cargo test --release --locked --test '*'
cargo bench --locked --bench load --no-run
python3 -I gen_compat.py --check

# ufbx-rust's tests as they are; this crate reads a stream to its end, so that test's `skip` is never called
cd ufbx-compat
ln -sfn "$rust/tests" tests
CARGO_TARGET_DIR=../target/ufbx-compat cargo test --locked -- --skip blender_default_custom_stream_skip
