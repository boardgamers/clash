#!/usr/bin/env bash

set -euo pipefail

VERSION=$1
if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

echo "Building server..."
./scripts/build-wasm-server.sh

echo "Publishing server..."
sed -i 's#"name": "server"#"name": "@bgs/clash-server"#' pkg/package.json
sed -i "s#\"version\": \"0.1.0\"#\"version\": \"$VERSION\"#" pkg/package.json
pushd server
wasm-pack publish --access public
popd

echo "Done!"
