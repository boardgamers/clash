#!/bin/bash


cargo build --target wasm32-unknown-unknown
cd www
wasm-pack build --dev
export NODE_OPTIONS=--openssl-legacy-provider
npm run start

# env error
# wasm-dis ../target/wasm32-unknown-unknown/debug/client.wasm | less
