#!/usr/bin/env bash

set -euo pipefail

pushd server
wasm-pack build --target nodejs
sed -i 's#"name": "server"#"name": "@clash/server"#' pkg/package.json
popd

echo "Done!"
