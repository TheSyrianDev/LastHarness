# Last AI for Omarchy

This is its own package, `lastai-omarchy`, not the plain Arch one: it is compiled with the Omarchy integration,
so the "Omarchy" and "Omarchy (transparent)" looks take the colours of the current theme and change with
`omarchy theme set`, and the launcher icon is recoloured on each theme change. It installs the same commands,
so it replaces `lastai` rather than sitting beside it.

Install the Omarchy package from the release, then wire the extras for your user (no root):

    sudo pacman -U LastAI-omarchy-*.pkg.tar.zst
    lastai-omarchy-setup

Install a newer package the same way to update. `lastai-omarchy-setup --uninstall` removes the theme hook and
icons for your user (then `sudo pacman -R lastai`; your settings and chats are kept), and `--dry-run` shows
what it would do. To build the package yourself, run `makepkg -si` in `packaging/aur`.

Or, on Omarchy, let the installer pick it: `curl -fsSL https://thesyriandev.github.io/lastsoft/install.sh | sh`
reads `/etc/os-release`, sees Omarchy, and installs this package instead of the plain Arch one.

`lastai-omarchy-setup` never edits your Hyprland config. For the see-through look, add this line yourself:

    o.window("lastai-gui", { opacity = "0.85 override 0.85 override" })

From a source checkout without the package, `scripts/install-omarchy.sh` installs into your home folder instead.
