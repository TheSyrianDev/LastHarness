# Milestone 1 report: core and terminal chat

Written by the build session while the user was away. What exists, how it was checked, and what is left.

## What exists

Rust workspace in `crates/`, seven crates:

| Crate | What it does |
|---|---|
| `lh-provider` | Talks to OpenAI-style servers: streaming chat, native tool calls, tolerant parsing (checked against the raw captures of all 7 models from spike 1), retries with plain-language errors, per-provider quirks, sampling and template settings, capability probing (context size, tools, vision from llama-swap and llama.cpp), image generation. |
| `lh-secrets` | Keys in the OS secret store (tested against gnome-keyring); config holds only `secret:NAME`. No plain-file fallback. |
| `lh-store` | One SQLite file: spend ledger and an append-only activity log (the database refuses edits and deletes), with undo information for file writes. |
| `lh-spend` | Prices, the three spending modes (no limits, hard caps, estimate-and-approve), and a meter that checks every model call or image before it is made and records it after. |
| `lh-tools` | The turn loop, six tools plus image generation, permission rules (tiered, ask-everything, bypass), a shell command classifier, web fetch with a guard against reaching local or private addresses, SearXNG search, and auto-compaction. |
| `lh-core` | The daemon `lhd`: local API over plain HTTP on loopback with SSE, bearer-token and Host-header checks, idle mode, sessions, activity and undo endpoints. |
| `lh-tui` | The terminal client `lh`. |

## How it was checked

92 automated tests pass and `cargo clippy` is clean across the workspace. Beyond tests:

- `lhd` and `lh` run together in a pseudo-terminal against the user's llama-swap: a refused permission left no file.
- Exit test, local half: Tiel-Coder-35B (through lhd, tools on) was asked to create `primes.py` and run it. It wrote the file, asked once to write and once to run, ran it, and answered "37" in 36 seconds. The activity log shows the startup settings, both permission answers and both tool calls; the spend ledger shows three model calls at zero cost (local).
- Idle mode: a release build of `lhd` uses about 7 MB (PSS). With the idle time running, and again once idle, 0 CPU ticks and no measurable thread wake-ups over 60 seconds. The idle manager is a single task: one timer while awake, no timer while idle, and a running turn keeps the core awake.
- Capability probe against llama-swap: LFM2.5 reports a 32,768-token context, tools yes, vision no.

## What is not done

- The cloud half of the exit test, Ollama and LM Studio runs, and probing for those servers.
- Tray icon, start at login, paired-device wake-up (pairing is Milestone 6/9), video generation, paid search providers.
- Details are in `.grill/parked.md`.

## Choices I made that the spec does not cover

Listed in `.grill/parked.md` under Milestone 1: shell commands that cannot be classified ask every time; unpriced paid models ask or are refused; spend limits use UTC days; compaction runs between turns only and starts at 75% of the reported context size (24,000 tokens if unknown).
