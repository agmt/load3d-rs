#!/bin/sh
# UFBX_PATH names ufbx's clone, ../ufbx beside this repository unless set; a missing one is cloned at the release the
# oracle compares with.
set -eux

root=$(cd "$(dirname "$0")" && pwd)
ufbx=${UFBX_PATH:-$root/../ufbx}
if [ ! -d "$ufbx/data" ]; then
    version=$(cd "$root/oracle" && cargo run --release --locked -q -- ufbx-version)
    git clone --depth 1 --branch "$version" https://github.com/ufbx/ufbx "$ufbx"
fi
UFBX_DATA=$(cd "$ufbx/data" && pwd)
export UFBX_DATA

cd "$root"
./test.sh
examples/viewer/test.sh
cargo test --release --locked --lib -- --ignored

cd oracle
cargo test --release --locked --lib --bins
cargo run --release --locked -- all --corpus "$UFBX_DATA"
cargo test --release --locked --test '*'
