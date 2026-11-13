#!/bin/sh
# Copy committed files from the development repo into the public copy, leaving out private material.
# The public copy has its own history: one commit per sync, never the development history.
#
#   scripts/sync-public.sh [--apply] [--rev REV] [public-folder]
#
#   public-folder  default: ../public/LastHarness
#   --rev REV      the development commit to copy from (default: HEAD)
#   --apply        write the files and commit in the public copy. Without it nothing is written:
#                  the script only shows what would change (a dry run).
#
# Only files committed in REV are copied, never uncommitted work. A file that exists only in the public
# copy is kept, unless it was once tracked in development and has since been removed there.
set -eu
dev="$(cd "$(dirname "$0")/.." && pwd)"
apply=0
rev=HEAD
pub=""
while [ $# -gt 0 ]; do
  case "$1" in
    --apply) apply=1 ;;
    --dry-run) apply=0 ;;
    --rev) rev="$2"; shift ;;
    -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
    *) pub="$1" ;;
  esac
  shift
done
pub="${pub:-$dev/../public/LastHarness}"
rev="$(git -C "$dev" rev-parse --verify "$rev^{commit}")"
short="$(git -C "$dev" rev-parse --short "$rev")"

# What stays private. Paths are relative to the repo root.
EXCLUDE='^(research/|\.grill/|spikes/|docs/|brand/|PRODUCT\.md$|AGENTS\.md$|CLAUDE\.md$|scripts/sync-public\.sh$|scripts/public-CONTRIBUTING\.md$|scripts/public-scan-allow$|\.claude/)'
# Text that must never reach the public copy: private paths, tokens and keys.
SECRETS='omar/Projects|/run/media/omar|/home/omar|harvest-instagram|ghp_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9_-]{20,}|sk-ant-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|xox[abposr]-[A-Za-z0-9-]{10,}|-----BEGIN [A-Z ]*PRIVATE KEY-----'

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT INT TERM
list="$work/files"
# the README's logo is the one brand file that goes public
{ git -C "$dev" ls-tree -r --name-only "$rev" | grep -Ev "$EXCLUDE"; git -C "$dev" ls-tree -r --name-only "$rev" -- 'brand/assets/' | grep -E '^brand/assets/lastharness-lockup-on-[^/]*\.svg$'; } | sort -u > "$list"

# uncommitted edits to synced files would be silently left out; make the user decide first
dirty="$(cd "$dev" && git status --porcelain --untracked-files=no | cut -c4- | grep -Fxf "$list" || true)"
if [ -n "$dirty" ]; then
  echo "sync-public: these synced files have uncommitted changes in development (they would not be copied):" >&2
  echo "$dirty" | sed 's/^/  /' >&2
  if [ "$apply" = 1 ]; then
    echo "sync-public: commit or stash them first" >&2
    exit 1
  fi
fi

# the new public tree, exactly as committed in REV
new="$work/tree"
mkdir -p "$new"
tr '\n' '\0' < "$list" | xargs -0 git -C "$dev" archive --format=tar "$rev" -- | tar -x -C "$new"
git -C "$dev" show "$rev:scripts/public-CONTRIBUTING.md" > "$new/CONTRIBUTING.md" 2>/dev/null || rm -f "$new/CONTRIBUTING.md"
echo CONTRIBUTING.md >> "$list"

# nothing private may slip through
# (scripts/public-scan-allow lists known fake test values as path:match lines; they may pass)
allow="$work/allow"
git -C "$dev" show "$rev:scripts/public-scan-allow" 2>/dev/null | grep -v '^#' > "$allow" || true
found="$( (grep -rIoE "$SECRETS" "$new" || true) | sed "s|^$new/||" | sort -u | grep -Fvxf "$allow" || true)"
if [ -n "$found" ]; then
  echo "sync-public: private paths or secrets found; nothing was synced:" >&2
  echo "$found" | sed 's/^/  /' >&2
  exit 1
fi

# removed in development: paths tracked at some point up to REV that REV no longer has
gone="$work/gone"
git -C "$dev" log --no-renames --diff-filter=D --format= --name-only "$rev" | sort -u | grep -v '^$' | grep -Fvxf "$list" > "$gone" || true

if [ -d "$pub/.git" ]; then
  if [ -n "$(git -C "$pub" status --porcelain)" ]; then
    echo "sync-public: $pub has uncommitted changes; commit or discard them first" >&2
    exit 1
  fi
  git -C "$pub" ls-files | sort > "$work/pubfiles"
else
  : > "$work/pubfiles"
fi
del="$work/del"
grep -Fxf "$gone" "$work/pubfiles" > "$del" || true

echo "sync-public: from development $short into $pub"
if [ "$apply" = 0 ]; then
  added=0 changed=0
  while IFS= read -r f; do
    if [ ! -e "$pub/$f" ]; then echo "  add     $f"; added=$((added + 1))
    elif ! cmp -s "$new/$f" "$pub/$f"; then echo "  change  $f"; changed=$((changed + 1)); fi
  done < "$list"
  while IFS= read -r f; do echo "  delete  $f"; done < "$del"
  kept="$(grep -Fvxf "$list" "$work/pubfiles" | grep -Fvxf "$del" || true)"
  if [ -n "$kept" ]; then echo "$kept" | sed 's/^/  keep (public only)  /'; fi
  echo "sync-public: dry run: $added to add, $changed to change, $(wc -l < "$del") to delete. Nothing was written. Use --apply to sync."
  if [ "$changed" -gt 0 ] && [ -d "$pub" ]; then
    echo "--- diff (public copy, then development $short) ---"
    while IFS= read -r f; do
      [ -e "$pub/$f" ] && ! cmp -s "$new/$f" "$pub/$f" && diff -u --label "public/$f" --label "dev/$f" "$pub/$f" "$new/$f" || true
    done < "$list"
  fi
  exit 0
fi

if [ ! -d "$pub/.git" ]; then
  mkdir -p "$pub"
  git -C "$pub" init -q -b main
fi
while IFS= read -r f; do rm -f "$pub/$f"; done < "$del"
cp -a "$new/." "$pub/"
find "$pub" -mindepth 1 -type d -empty -not -path "$pub/.git*" -delete

git -C "$pub" add -A
if git -C "$pub" diff --cached --quiet; then
  echo "sync-public: nothing new since the last sync"
  exit 0
fi
# the public commit carries the public pseudonym and its GitHub noreply address, never the owner's own identity
uid="$(gh api user --jq .id)"
pub_name=TheSyrianDev
pub_email="$uid+TheSyrianDev@users.noreply.github.com"
GIT_AUTHOR_NAME="$pub_name" GIT_AUTHOR_EMAIL="$pub_email" GIT_COMMITTER_NAME="$pub_name" GIT_COMMITTER_EMAIL="$pub_email" \
  git -C "$pub" -c user.name="$pub_name" -c user.email="$pub_email" commit -q -m "Sync from development ($short)"
echo "sync-public: committed $(git -C "$pub" rev-parse --short HEAD) in $pub (from $short)"
