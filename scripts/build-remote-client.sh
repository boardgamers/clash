#!/usr/bin/env bash

set -euo pipefail

# from https://gist.github.com/nicolas-sabbatini/8af10dddc96be76d2bf24fc671131add

HELP_STRING=$(
	cat <<-END
		usage: build_remote.sh [--release]
	END
)

die() {
	echo >&2 "$HELP_STRING"
	echo >&2
	echo >&2 "Error: $*"
	exit 1
}

RELEASE=no
# Parse primary commands
while [[ $# -gt 0 ]]; do
	key="$1"
	case $key in
	--release)
		RELEASE=yes
		shift
		;;

	-h | --help)
		echo "$HELP_STRING"
		exit 0
		;;

	*)
		POSITIONAL+=("$1")
		shift
		;;
	esac
done

# Restore positionals
set -- "${POSITIONAL[@]}"
[ $# -ne 0 ] && die "too many arguments provided"

pushd client

PROJECT_NAME=remote_client

TARGET_DIR="../target/wasm32-unknown-unknown"
# Build
echo "Building $PROJECT_NAME..."
if [ "$RELEASE" == "yes" ]; then
	cargo build --release --target wasm32-unknown-unknown
	TARGET_DIR="$TARGET_DIR/release"
else
	cargo build --target wasm32-unknown-unknown
	TARGET_DIR="$TARGET_DIR/debug"
fi

# Generate bindgen outputs
echo "Running wasm-bindgen..."

mkdir -p dist

DEBUG_ARG=""
if [ "$RELEASE" != "yes" ]; then
  DEBUG_ARG="--keep-debug --debug"
fi
wasm-bindgen $TARGET_DIR/"$PROJECT_NAME".wasm --out-dir dist $DEBUG_ARG --target web --no-typescript

echo "Patching wasm-bindgen output..."

# Shim to tie the thing together
sed -i "s/import \* as __wbg_star0 from 'env';//" dist/"$PROJECT_NAME".js
sed -i "s/import \* as __wbg_star1 from 'env';//" dist/"$PROJECT_NAME".js
sed -i "s/let wasm;/let wasm; export const set_wasm = (w) => wasm = w;/" dist/"$PROJECT_NAME".js
sed -i "s/imports\['env'\] = __wbg_star0;/return imports.wbg\;/" dist/"$PROJECT_NAME".js
sed -i "s/imports\['env'\] = __wbg_star1;/return imports.wbg\;/" dist/"$PROJECT_NAME".js
sed -i "s/const imports = __wbg_get_imports();/return __wbg_get_imports();/" dist/"$PROJECT_NAME".js

pushd js
mkdir -p dist
rm -rf dist/*
npm install
npm run build
cp -r ../assets dist/
cp package.json dist/
pushd dist
mv *.wasm client.wasm
popd
popd # js

popd # client

echo "Done!"
