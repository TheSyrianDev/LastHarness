#!/bin/sh
# Copy the development repo into the public copy, leaving out private material, and commit it there.
# The public copy has its own history: one commit per sync, never the development history.
#
#   scripts/sync-public.sh [public-folder]     (default: ../LastHarness-public)
#
# Only files tracked by git are copied, as they are in the last commit (HEAD), never uncommitted work.
set -eu
dev="$(cd "$(dirname "$0")/.." && pwd)"
pub="${1:-$dev/../LastHarness-public}"

# What stays private. Paths are relative to the repo root.
EXCLUDE='^(research/|\.grill/|spikes/|docs/archive/|docs/for-omar\.md$|AGENTS\.md$|CLAUDE\.md$|scripts/sync-public\.sh$|scripts/public-CONTRIBUTING\.md$|\.claude/)'

if [ ! -d "$pub/.git" ]; then
  mkdir -p "$pub"
  git -C "$pub" init -q -b main
fi
if [ -n "$(git -C "$pub" status --porcelain)" ]; then
  echo "sync-public: $pub has uncommitted changes; commit or discard them first" >&2
  exit 1
fi

# start from an empty tree so files deleted in development disappear here too
git -C "$pub" ls-files -z | (cd "$pub" && xargs -0 -r rm -f)
git -C "$dev" ls-files | grep -Ev "$EXCLUDE" | while IFS= read -r f; do
  mkdir -p "$pub/$(dirname "$f")"
  git -C "$dev" show "HEAD:$f" > "$pub/$f"
  if [ -x "$dev/$f" ]; then chmod +x "$pub/$f"; fi
done
cp "$dev/scripts/public-CONTRIBUTING.md" "$pub/CONTRIBUTING.md" 2>/dev/null || true
find "$pub" -mindepth 1 -type d -empty -not -path "$pub/.git*" -delete

# nothing private may slip through
if grep -rIl -E 'omar/Projects|/run/media/omar|harvest-instagram' "$pub" --exclude-dir=.git >/dev/null; then
  echo "sync-public: private paths found in the public copy:" >&2
  grep -rIl -E 'omar/Projects|/run/media/omar|harvest-instagram' "$pub" --exclude-dir=.git >&2
  exit 1
fi

rev="$(git -C "$dev" rev-parse --short HEAD)"
git -C "$pub" add -A
if git -C "$pub" diff --cached --quiet; then
  echo "sync-public: nothing new since the last sync"
  exit 0
fi
git -C "$pub" -c user.name="$(git -C "$dev" config user.name)" -c user.email="$(git -C "$dev" config user.email)" commit -q -m "Sync from development ($rev)"
echo "sync-public: committed $(git -C "$pub" rev-parse --short HEAD) in $pub (from $rev)"
