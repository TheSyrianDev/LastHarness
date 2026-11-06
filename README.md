<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lastharness-lockup-on-dark.svg">
    <img alt="LastHarness" src=".github/assets/lastharness-lockup-on-light.svg" width="360">
  </picture>
</p>

<p align="center"><em>The last harness you'll need.</em><br>
A local-first AI harness built around a team: departments, and the workers in them.</p>

> [!WARNING]
> **Alpha.** LastHarness is early software and is only used on one machine so far. Expect rough edges and breaking changes. LastHarness (the desktop app and the `lh` terminal app) is tested on Linux (Omarchy) and Windows (author-tested). The macOS build is early alpha: it starts in CI, but no one has used it day to day yet.

<p align="center">
  <a href="https://github.com/TheSyrianDev/LastHarness/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/TheSyrianDev/LastHarness/actions/workflows/ci.yml/badge.svg"></a>
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

This makes four programs in `target/release/`: `lhd` (the core), `lh` and `lastharness` (the same terminal client under two names) and `lastharness-gui` (the desktop app). The desktop app takes a few minutes to build the first time; `cargo build --release -p lh-core -p lh-tui` builds only the core and the terminal.

## First run

```
target/release/lh setup        # twelve plain questions; Enter accepts each default
target/release/lhd             # start the core (it reads the settings file)
target/release/lh              # chat
target/release/lastharness-gui # or the desktop app (needs the core running)
```

The desktop app finds the core and its token by itself (flags: `--base`, `--token-file`). It has plain default looks you can change: put a `.toml` file in `~/.config/lastharness/themes/` (the six built-in ones, three families each in light and dark, are in `crates/lh-gui/themes/` as examples). Press Ctrl+N for a new chat.

The Omarchy release (`LastHarness-<version>-omarchy-x86_64.tar.gz`, installed with its `install-omarchy.sh`) also offers the desktop's own look, and an "Omarchy (transparent)" look that is liquid glass in the Omarchy colours. The transparent look is as see-through as the Hyprland window rule for the app's window (class `lastharness-gui`) says; without a rule it stays opaque. Other builds (AppImage, Windows, macOS, or a source build with `--no-default-features`) leave the Omarchy looks out. To add one, put this line in `~/.config/hypr/hyprland.lua` (0.85 is how much of the window shows):

```lua
o.window("lastharness-gui", { opacity = "0.85 override 0.85 override" })
```

To start the core at every login: `lh service preview`, then `lh service install` (a systemd user service; no sudo; closing the terminal never stops the core).

## Commands

Every command has `help`, which works without the core running, for example `lh team help`.

- `lhd` is the core. It listens on `127.0.0.1:7842` only. Settings come from `config.toml`; a command-line flag overrides the file. The token the clients need is written to `$XDG_RUNTIME_DIR/lastharness/token`.
- `lh` and `lastharness` open the terminal chat. Options: `--url`, `--token-file`, `--model`, `--ask-all`, `--incognito`.
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

- Settings: `~/.config/lastharness/config.toml` (optional spending prices in `spend.toml` beside it).
- Data: `~/.local/share/lastharness/`: `lh.db` (activity log, rules, indexes), `memory/` (your notes as Markdown), `skills/`, `team/` (roles and departments), `mcp.json`, `web/` (the browser door's certificate).
- Token: `$XDG_RUNTIME_DIR/lastharness/token`.

## Not built yet

- A phone app, and any final visual design (no logo, name styling or brand colours yet). The browser page and the desktop app's default look are plain on purpose.
- macOS; installers; signed updates; crash reports are only prepared, not sent.
- The hosted relay, subscription and memory sync.
- Agents loading and unloading local models (they can already list models, pick one among your providers and propose a new provider for you to approve).
- Installing a plugin from a link (folders only), and project-level skills.
- No messaging channel has been tried against a real account; they are tested against mock services.

Tests: `cargo test --workspace` (the real embedding model test is skipped with `-- --skip real_embeddinggemma`). Licence: PolyForm Noncommercial 1.0.0 (see `LICENSE` and `COMMERCIAL-LICENSING.md`); `third_party/` keeps its own licences.
