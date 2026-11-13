#!/usr/bin/env bash
# Moves the checkout from the old folder name to the Last AI folder name.
# Written for the CEO to run by hand. It does NOT run itself and does not change the system files.
# Usage: scripts/rename-folder.sh ~/Projects/LastAI
set -euo pipefail
old="$(cd "$(dirname "$0")/.." && pwd)"
new="${1:?give the new folder path}"
[ -e "$new" ] && { echo "target exists: $new" >&2; exit 1; }
if [ -n "$(git -C "$old" status --porcelain)" ]; then
  echo "uncommitted changes in $old; commit or stash first" >&2; exit 1
fi
mv "$old" "$new"
echo "moved to $new"
echo "Next: relink ~/.local/bin (la, lastaid, lastai-gui) to $new/target/release, and fix the Hyprland and desktop paths if they point at the old folder."
