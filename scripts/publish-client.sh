#!/usr/bin/env bash

set -euo pipefail

VERSION=$1
if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

echo "Building client..."
./scripts/build-remote-client.sh # --release

echo "Publishing client..."
pushd client
sed -i 's#"name": "client"#"name": "@boardgamers/clash-client"#' pkg/package.json
sed -i "s#\"version\": \"0.1.0\"#\"version\": \"$VERSION\"#" pkg/package.json
npm publish --access public
popd

echo "Done!"
