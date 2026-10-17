#!/bin/sh
# Builds the demo into web/. Serve it with: python3 -m http.server -d web
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/whenabouts_demo.wasm web/
