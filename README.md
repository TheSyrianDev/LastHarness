# LastHarness

LastHarness is a local-first AI harness for one person's computer. A core program (`lhd`) holds the conversation, tools, memory and learning, and talks to any model server that speaks the OpenAI chat API (tested with llama.cpp through llama-swap). Front ends are thin clients of the core's local API: a terminal app, a browser page, and messaging channels.
It remembers across sessions (notes you can read and edit as Markdown), learns your habits as rules you approve, and can hand bigger jobs to departments of role workers that plan, run and check the work.
Sensitive notes and rules are used only with a model on this machine. Permissions are tiered: reads are free, writes ask once, and anything risky always asks.
It is not on GitHub or any hosted service yet, and it is Linux only so far.

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

The desktop app finds the core and its token by itself (flags: `--base`, `--token-file`). It has plain default looks you can change: put a `.toml` file in `~/.config/lastharness/themes/` (the six built-in ones, three families each in light and dark, are in `crates/lh-gui/themes/` as examples). Press Ctrl+N for a new chat; the keys for everything else are in `docs/milestone-8-report.md`.

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
- The design: `docs/digest.md` (one page), `docs/spec/lastharness.md`, `docs/roadmap.md`. Reports for each milestone are in `docs/`. Questions waiting for the owner are in `docs/for-omar.md` and `.grill/parked.md`.

## Not built yet

- A phone app, and any final visual design (no logo, name styling or brand colours yet). The browser page and the desktop app's default look are plain on purpose.
- macOS and Windows; installers; signed updates; crash reports are only prepared, not sent.
- The hosted relay, subscription and memory sync.
- Agents choosing or loading models and proposing new providers (the core talks to one endpoint).
- Installing a plugin from a link (folders only), and project-level skills.
- No messaging channel has been tried against a real account; they are tested against mock services.

Tests: `cargo test --workspace` (the real embedding model test is skipped with `-- --skip real_embeddinggemma`). Licence: Apache-2.0.
