#!/usr/bin/env bash

set -euo pipefail

VERSION=${1:-}
if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

echo "Building client..."
./scripts/build-remote-client.sh # --release

echo "Publishing client..."
pushd client/js/dist
sed -i "s#\"version\": \"0.1.0\"#\"version\": \"$VERSION\"#" package.json
npm publish --access public
popd

echo "Done!"
