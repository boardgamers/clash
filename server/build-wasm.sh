#!/usr/bin/env bash

set -e

wasm-pack build --target nodejs

echo "Done!"
