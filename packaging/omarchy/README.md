# Last AI for Omarchy

This build follows your Omarchy theme: the "Omarchy" and "Omarchy (transparent)" looks take the colours of the
current theme and change with `omarchy theme set`, and the launcher icon is recoloured on each theme change.

Install the Arch package from the release, then wire the Omarchy extras for your user (no root):

    sudo pacman -U LastAI-*.pkg.tar.zst
    lastai-omarchy-setup

Install a newer package the same way to update. `lastai-omarchy-setup --uninstall` removes the theme hook and
icons for your user (then `sudo pacman -R lastai`; your settings and chats are kept), and `--dry-run` shows
what it would do. To build the package yourself, run `makepkg -si` in `packaging/aur`.

`lastai-omarchy-setup` never edits your Hyprland config. For the see-through look, add this line yourself:

    o.window("lastai-gui", { opacity = "0.85 override 0.85 override" })

From a source checkout without the package, `scripts/install-omarchy.sh` installs into your home folder instead.
