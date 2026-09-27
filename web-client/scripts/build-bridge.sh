#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo build -p server --lib --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/server.wasm --target web --out-dir web-client/public/engine
wasm-bindgen target/wasm32-unknown-unknown/release/server.wasm --target nodejs --out-dir web-client/.engine
printf '{"type":"commonjs"}\n' > web-client/.engine/package.json
