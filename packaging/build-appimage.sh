#!/bin/sh
# Build LastHarness-x86_64.AppImage from a release build.
# Run on an old distro (the CI uses Ubuntu 22.04) so the glibc it needs is available on most systems.
# Needs: linuxdeploy in PATH or LINUXDEPLOY=/path/to/linuxdeploy-x86_64.AppImage
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build --release -p lh-gui -p lh-core
app="$root/target/AppDir"
rm -rf "$app"
mkdir -p "$app/usr/bin"
cp target/release/lastharness-gui target/release/lhd "$app/usr/bin/"
ld="${LINUXDEPLOY:-linuxdeploy}"
# GPU drivers (Vulkan, Mesa) are never bundled: the app uses the host's.
ARCH=x86_64 "$ld" --appdir "$app" \
  --desktop-file packaging/lastharness.desktop \
  --icon-file packaging/lastharness.png --icon-filename lastharness \
  --exclude-library 'libvulkan*' --exclude-library 'libGL*' --exclude-library 'libEGL*' \
  --output appimage
mkdir -p dist && mv LastHarness*.AppImage dist/ 2>/dev/null || mv ./*.AppImage dist/
ls -l dist
