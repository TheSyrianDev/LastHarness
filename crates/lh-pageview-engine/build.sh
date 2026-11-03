#!/bin/sh
# Build the page drawing helper and put it beside the app, where the app looks for it.
#   ./build.sh            release build, copied to ../../target/release/
#   ./build.sh debug      debug build, copied to ../../target/debug/
# The helper is its own cargo workspace (see Cargo.toml), so a plain `cargo build --workspace` does not build it.
set -e
cd "$(dirname "$0")"
profile=release
flag=--release
if [ "$1" = "debug" ]; then profile=debug; flag=; fi
nice -n 19 cargo build $flag
mkdir -p "../../target/$profile"
cp "target/$profile/lh-pageview-helper" "../../target/$profile/lh-pageview-helper"
echo "helper: ../../target/$profile/lh-pageview-helper"
