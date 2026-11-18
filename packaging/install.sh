#!/usr/bin/env bash
# Install Last AI from a built checkout into ~/.local/bin. No root needed. Run from the repository root.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${PREFIX:-$HOME/.local}/bin"
cargo build --release -p lh-core -p lh-tui -p lh-gui
mkdir -p "$BIN"
install -m755 "$ROOT/target/release/lastaid" "$BIN/lastaid"
install -m755 "$ROOT/target/release/lh" "$BIN/lh"
install -m755 "$ROOT/target/release/lastharness" "$BIN/lastharness"
install -m755 "$ROOT/target/release/lastai-gui" "$BIN/lastai-gui"
echo "installed to $BIN: lastaid, lh, lastharness, lastai-gui"
echo "make sure $BIN is on PATH"
