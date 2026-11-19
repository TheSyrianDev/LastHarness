<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="brand/assets/lastharness-lockup-on-dark.svg">
    <img alt="LastHarness" src="brand/assets/lastharness-lockup-on-light.svg" width="300">
  </picture>
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.ar.md">العربية</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a>
</p>

<p align="center"><a href="https://thesyriandev.github.io/LastSoft/">Part of LastSoft</a></p>


<p align="center"><em>The last harness you'll need.</em><br>
A local-first AI harness built around a team: departments, and the workers in them.</p>

> [!WARNING]
> **Alpha, version 0.1.0-alpha.** See [CHANGELOG.md](CHANGELOG.md) for what works and the known gaps. LastHarness is early software and is only used on one machine so far. Expect rough edges and breaking changes. The core, the terminal app (`lh`) and the desktop app are tested on Linux (Omarchy) and Windows (author-tested). The macOS build is early alpha: it starts in CI, but no one has used it day to day yet.

<p align="center">
  <a href="https://github.com/TheSyrianDev/LastHarness/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: PolyForm Noncommercial 1.0.0" src="https://img.shields.io/badge/license-PolyForm--Noncommercial--1.0.0-blue"></a>
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange">
</p>

LastHarness is a local-first AI harness for one person's computer. A core program (`lhd`) holds the conversation, the team (departments and their workers), tools, memory and learning, and is model-agnostic, with no model lock-in. It works with any model or provider: local servers such as llama-swap, llama.cpp, Ollama and LM Studio, or any OpenAI-compatible or Anthropic-style cloud API (tested so far with llama.cpp through llama-swap). Front ends are thin clients of the core's local API: a terminal app, a browser page, and messaging channels.
It remembers across sessions (notes you can read and edit as Markdown), learns your habits as rules you approve, and can hand bigger jobs to departments of role workers that plan, run and check the work.
Sensitive notes and rules are used only with a model on this machine. Permissions are tiered: reads are free, writes ask once, and anything risky always asks.
It runs on Linux and Windows; macOS is early alpha.

## Build

```
cargo build --release
```

This makes these programs in `target/release/`: `lhd` (the core, also built as `lastaid`), `lh` and `lastharness` (the same terminal client under two names) and `lastharness-gui` (the desktop app, also built as `lastai-gui`). The desktop app takes a few minutes to build the first time; `cargo build --release -p lh-core -p lh-tui` builds only the core and the terminal.

## First run

```
lh setup                       # twelve plain questions; Enter accepts each default
lhd                            # start the core (it reads the settings file)
lh                             # chat
lastharness-gui                # or the desktop app (needs the core running)
```

The desktop app finds the core and its token by itself (flags: `--base`, `--token-file`). It has plain default looks you can change: a look is a folder in `~/.config/lastai/themes/` with a `theme.toml` inside (and a `background.png` beside it if you want a picture), and the folder's name is the look's name. [docs/looks.md](docs/looks.md) lists every key; the built-in looks, each family in light and dark, are in `crates/lh-gui/themes/` as examples. Press Ctrl+N for a new chat.

The Omarchy edition of the desktop app (theme following, a see-through look in the desktop's colours, its own Arch package and setup script) lives on the `omarchy` branch of this repository, not here. This branch is the harness on its own: the same themes, `theme.toml` and folders, no desktop coupling.

Check a built core without the desktop: `scripts/smoke-headless.sh` (starts `lhd` on a temp store and checks the HTTP API).

To start the core at every login: `lh service preview`, then `lh service install` (a systemd user service; no sudo; closing the terminal never stops the core).

## Providers and models

Providers are the servers that answer chat: a local llama-swap or llama.cpp, Ollama, LM Studio, or an OpenAI-compatible or Anthropic-style cloud API. Add and test them on the Models and keys page of the desktop app, or with `lh settings`. API keys are read from environment variables you name; nothing is sent to a provider you have not added.

## Create (images and video)

The Create block on the Models and keys page makes images (Z-Image Turbo) and short videos (Wan 2.2 TI2V-5B, MP4) through a local ComfyUI on `127.0.0.1:8188`. Type a prompt, choose Image or Video, press Generate. The job list shows progress and, when it is done, the result inline (the image, or the first frame of the video). Files are saved in `~/AI/Create/`. Image and video jobs take the machine lock and free GPU memory afterwards, so they wait while another session holds the lock. ComfyUI and its model files are not part of this repository.

## Web search

`web_search` uses a SearXNG instance. If the settings name none, LastHarness looks for one on `127.0.0.1:8888`. If nothing answers there, the tool says so and suggests starting SearXNG or setting its address.

## Install and release build

Release downloads (prereleases on GitHub):

- Debian / Ubuntu: `sudo apt install ./LastAI-<version>-amd64.deb`.
- Fedora / openSUSE: `sudo dnf install ./LastAI-<version>-x86_64.rpm`.
- Arch: `sudo pacman -U LastAI-*.pkg.tar.zst`. The Omarchy edition has its own package on the `omarchy` branch.
- Other Linux distributions: the AppImage.
- Windows: the `LastAI-<version>-windows-x64.zip`.
- macOS (Apple silicon, early alpha): the `LastAI-<version>-macos-arm64.zip`. Unzip it and drag `Last AI.app` to Applications; it is not signed, so the first launch is right-click → Open.

Build it yourself:

- Source: `cargo build --release` (needs Rust stable; on Linux the desktop app needs the usual GPUI and system libraries).
- Arch package: `makepkg -si` in `packaging/aur`.
- Linux AppImage: `packaging/build-appimage.sh` (needs `linuxdeploy`; see the script's header).
- .deb / .rpm: `packaging/nfpm.yaml` (see its header).

## Commands

Every command has `help`, which works without the core running, for example `lh team help`.

- `lastaid` is the core. It listens on `127.0.0.1:7842` only. Settings come from `config.toml`; a command-line flag overrides the file. The token the clients need is written to `$XDG_RUNTIME_DIR/lastai/token`.
- `lh` and `lastharness` open the terminal chat. Options: `--url`, `--token-file`, `--model`, `--ask-all`, `--incognito`. In the chat, `/review` asks for suggestions from the git diff (or from the chat when there is no diff), and `/review transcript` always uses the chat.
- `lh setup`: first-run questions. `lh settings`: see and change every setting (`set`, `reset`).
- `lh memory ...`: the notes the assistant keeps. `lh learn ...`: rules, style, skills and suggestions waiting for you.
- `lh team ...`: roles, model types, departments, giving a goal to a department (`lh team run`), workers, away mode, plugin folders.
- `lh mcp ...` and `lh trust <folder>`: MCP servers (nothing runs until you approve) and which project folders may bring their own.
- `lh devices ...`: pair browsers (`lh devices pair`, then `lh devices accept <PIN>`), list and remove them.
- `lh channels ...`: Telegram, Matrix, Slack and Signal channels (all off until you switch them on).
- `lh export <folder>` and `lh import <folder>`: move memory, rules, style, roles and departments.
- `lh service ...`: start the core at login.

The browser page is served over HTTPS when `web.listen` is set (for example `lh settings set web.listen 127.0.0.1:7843`); a browser pairs with a PIN first.

## Where files live

- Settings: `~/.config/lastai/config.toml` (optional spending prices in `spend.toml` beside it).
- Data: `~/.local/share/lastai/`: `lh.db` (activity log, rules, indexes), `memory/` (your notes as Markdown), `skills/`, `team/` (roles and departments), `mcp.json`, `web/` (the browser door's certificate).
- Token: `$XDG_RUNTIME_DIR/lastai/token`.

## Release

The 0.1.0-alpha release checklist is in `STATUS.md` in the Projects folder above this repo. Nothing is pushed or tagged from it yet.

## Not built yet

- A phone app, and any final visual design (no logo, name styling or brand colours yet). The browser page and the desktop app's default look are plain on purpose.
- macOS; installers; signed updates; crash reports are only prepared, not sent.
- The hosted relay, subscription and memory sync.
- Agents loading and unloading local models (they can already list models, pick one among your providers and propose a new provider for you to approve).
- Installing a plugin from a link (folders only), and project-level skills.
- No messaging channel has been tried against a real account; they are tested against mock services.

Tests: `cargo test --workspace` (the real embedding model test is skipped with `-- --skip real_embeddinggemma`). Licence: PolyForm Noncommercial 1.0.0 (see `LICENSE` and `COMMERCIAL-LICENSING.md`); `third_party/` keeps its own licences.
