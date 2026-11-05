# LastHarness for Omarchy

This build follows your Omarchy theme: the "Omarchy" and "Omarchy (transparent)" looks take the colours of the
current theme and change with `omarchy theme set`, and the launcher icon is recoloured on each theme change.

Install (into your home folder, no root):

    tar xzf LastHarness-<version>-omarchy-x86_64.tar.gz
    cd LastHarness-<version>-omarchy-x86_64
    ./install-omarchy.sh

Run it again to update. `./install-omarchy.sh --uninstall` removes it (your settings and chats are kept),
and `--dry-run` shows what it would do.

The script never edits your Hyprland config. For the see-through look, add this line yourself:

    o.window("lastharness-gui", { opacity = "0.85 override 0.85 override" })
