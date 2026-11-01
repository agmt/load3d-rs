#!/bin/sh
set -eux
cd "$(dirname "$0")"

cargo test --locked --features flatten
cargo test --locked --no-default-features
# a format's code compiled without its feature would be unused
for f in format-fbx format-obj format-3ds; do
    RUSTFLAGS="-D warnings" cargo build --locked --no-default-features --features "$f"
done
cargo build --locked --release --target wasm32-unknown-unknown
cargo package --locked --allow-dirty
