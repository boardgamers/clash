#!/usr/bin/env bash

set -euo pipefail

pushd server
wasm-pack build --target nodejs
popd

echo "Done!"
