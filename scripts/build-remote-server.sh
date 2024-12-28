#!/usr/bin/env bash

set -euo pipefail

RELEASE=no

# Parse primary commands
while [[ $# -gt 0 ]]; do
	key="$1"
	case $key in
	--release)
		RELEASE=yes
		shift
		;;

	*)
		POSITIONAL+=("$1")
		shift
		;;
	esac
done

ARGS="--dev --debug"
if [ "$RELEASE" = "yes" ]; then
  ARGS=""
fi

echo "Building server with release=$RELEASE..."
pushd server
wasm-pack build $ARGS --target nodejs
popd

echo "Done!"
