#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
CARGO_PROFILE_RELEASE_DEBUG=0 cargo build -p server --lib --locked --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/server.wasm --target web --remove-name-section --remove-producers-section --out-dir web-client/.bridge
wasm-bindgen target/wasm32-unknown-unknown/release/server.wasm --target nodejs --remove-name-section --remove-producers-section --out-dir web-client/.engine
printf '{"type":"commonjs"}\n' > web-client/.engine/package.json
