#!/bin/sh
set -eux
cd "$(dirname "$0")"

cargo test --locked
cargo test --locked --no-default-features
for f in format-fbx format-obj; do
    cargo build --locked --no-default-features --features "$f"
done
cargo build --locked --release --target wasm32-unknown-unknown
cargo package --locked
