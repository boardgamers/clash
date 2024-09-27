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
pushd server
wasm-pack publish --access public --tag "$VERSION"
popd

echo "Done!"
