#!/bin/sh
set -eux
cd "$(dirname "$0")"

cargo test --locked
cargo build --locked --release --target wasm32-unknown-unknown
mkdir -p target/site
cp index.html viewer.js target/wasm32-unknown-unknown/release/load3d_viewer.wasm target/site/

# Playwright's image carries Chromium and its libraries; the package must be the image's own version
docker run --rm -v "$PWD:/src:ro" mcr.microsoft.com/playwright:v1.64.1-resolute sh -ec '
    mkdir /test && cd /test
    cp -r /src/target/site /src/test.mjs .
    npm install --no-save --no-audit --no-fund playwright@1.64.1
    node test.mjs'
