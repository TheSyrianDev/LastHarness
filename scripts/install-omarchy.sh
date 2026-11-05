#!/bin/bash
# Install LastHarness for Omarchy into your home folder (no root needed).
#   ./install-omarchy.sh             install or update (safe to run again)
#   ./install-omarchy.sh --uninstall remove what this script installed
#   ./install-omarchy.sh --dry-run   show what would happen, change nothing
# It never edits your Hyprland config; it prints the one window rule for you to add.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
apps="$data/applications"
hook_dir="${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/hooks/theme-set.d"
bins=(lastharness-gui lhd lh lastharness)

dry=0; mode=install
for a in "$@"; do
  case "$a" in
    --uninstall) mode=uninstall ;;
    --dry-run) dry=1 ;;
    -h|--help) sed -n '2,6p' "$0"; exit 0 ;;
    *) echo "unknown option: $a" >&2; exit 2 ;;
  esac
done

run() { if (( dry )); then echo "would run: $*"; else "$@"; fi; }

# templates sit next to the script in the release, or in packaging/omarchy in the source tree
find_file() {
  local f
  for f in "$here/$1" "$here/omarchy/$1" "$here/../packaging/omarchy/$1" "$here/../packaging/$1"; do
    [[ -f $f ]] && { echo "$f"; return; }
  done
  echo "missing $1 next to $0" >&2; exit 1
}
find_bin() {
  local f
  for f in "$here/$1" "$here/../target/release/$1"; do
    [[ -x $f ]] && { echo "$f"; return; }
  done
  return 0
}

if [[ $mode == uninstall ]]; then
  for b in "${bins[@]}" lastharness-icon-refresh; do
    [[ -e $bin_dir/$b ]] && run rm -f "$bin_dir/$b"
  done
  [[ -e $apps/lastharness.desktop ]] && run rm -f "$apps/lastharness.desktop"
  [[ -e $hook_dir/lastharness-icon ]] && run rm -f "$hook_dir/lastharness-icon"
  for f in "$data"/icons/hicolor/*/apps/lastharness.{svg,png}; do
    [[ -e $f ]] && run rm -f "$f"
  done
  echo "LastHarness removed. Your settings and chats in ~/.config and ~/.local/share are kept."
  echo "If you added the Hyprland window rule for lastharness-gui, you can remove it yourself."
  exit 0
fi

found=0
for b in "${bins[@]}"; do
  src="$(find_bin "$b")"
  [[ -n $src ]] || continue
  run install -Dm755 "$src" "$bin_dir/$b"; found=1
done
(( found )) || { echo "no LastHarness binaries found next to $0" >&2; exit 1; }

run install -Dm755 "$(find_file lastharness-icon-refresh)" "$bin_dir/lastharness-icon-refresh"
run install -Dm755 "$(find_file theme-set-hook)" "$hook_dir/lastharness-icon"

desktop="$(mktemp)"; trap 'rm -f "$desktop"' EXIT
sed "s|^Exec=.*|Exec=$bin_dir/lastharness-gui|" "$(find_file lastharness.desktop)" > "$desktop"
grep -q '^StartupWMClass=' "$desktop" || echo 'StartupWMClass=lastharness-gui' >> "$desktop"
run install -Dm644 "$desktop" "$apps/lastharness.desktop"

if command -v rsvg-convert >/dev/null; then
  run "$bin_dir/lastharness-icon-refresh" || echo "icon refresh failed; it runs again on the next theme change" >&2
else
  echo "rsvg-convert not found; install librsvg for a theme-coloured icon" >&2
fi
command -v update-desktop-database >/dev/null && run update-desktop-database "$apps" || true

cat <<'MSG'

LastHarness is installed. For the see-through "Omarchy (transparent)" look, add this line to your
Hyprland config yourself (this script does not touch it):

  o.window("lastharness-gui", { opacity = "0.85 override 0.85 override" })
MSG
