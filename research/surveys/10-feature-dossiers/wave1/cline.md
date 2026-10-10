# Cline — Feature Dossier
Registry: #93 | Category: 01 | Verified: ~66.7k stars, Apache-2.0, last push 2026-08-22 (GitHub API 2026-08-23)

## Feature inventory
- Four product surfaces from one open-source agent core: VS Code extension, JetBrains plugin, CLI (`npm i -g cline`, interactive chat or fully headless), and web-based Kanban multi-agent board (separate `cline/kanban` repo) [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Human-in-the-loop approval: every file edit and terminal command requires approval by default; optional auto-approve toggle for autonomy [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Diff review + checkpoints: every edit surfaces as a reviewable/modifiable/revertible diff; all changes tracked with checkpoints for undoing agent work [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Plan/Act mode split: Plan mode explores codebase, asks clarifying questions, lays out strategy; Act mode executes after alignment [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Project rules + skills: `.clinerules` files define coding standards/conventions picked up automatically across CLI/VS Code/JetBrains; skills let the model load specific rules when needed [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Provider-agnostic models incl. local: Ollama / LM Studio / any OpenAI-compatible self-hosted endpoint alongside Anthropic/OpenAI/Google/Bedrock/OpenRouter etc. [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- MCP servers + plugin system: SDK registers tools and lifecycle hooks programmatically for logging, auditing, **policy enforcement**, domain capabilities; MCP for external systems; `cline mcp` management in CLI [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Multi-agent teams: coordinator agent decomposes work and delegates to specialist agents with own tools/context; team state persists across sessions (`cline --team-name …`) [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Scheduled agents: cron-driven recurring automations (PR summaries, dependency checks) that persist across restarts and run independent of any terminal session [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Messaging-platform connectors: Telegram, Slack (webhook or socket mode), Discord, Google Chat, WhatsApp, Linear — each conversation thread maps to an agent session with access control [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Headless JSON output for CI/CD: pipe input/output, jq-filterable event stream, chainable commands [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- Background-process awareness: watches long-running processes (dev servers) and reacts to new output, catching compile errors/test failures as they happen [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]
- SDK as engine substrate: same core powers CLI/Kanban/extensions; custom tools via `createTool` with typed input schemas [VERIFIED-fetched:https://raw.githubusercontent.com/cline/cline/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Default-on per-action approval gates w/ opt-in auto-approve | ⚙ Feature adoption | Kernel approval gate § | Deny-by-default execution permissioning is exactly the LastHarness invariant, proven at scale |
| Checkpoints + revertible diffs of every change | ⚙ Feature adoption | Decision-log § / Ops rollback | Complements immutable decision log with reversible workspace state |
| Plan/Act two-phase mode | ⚙ Feature adoption | Worker protocol § | Propose-then-execute keeps kernel in control of what actually runs |
| `.clinerules` + skills as portable project config files | ⚙ Feature adoption | Skills/plugins dept | Declarative, versionable, department-scoped behavior packs |
| Local-model support (Ollama/LM Studio/OpenAI-compatible) | ⚙ Feature adoption | Local inference dept | Directly satisfies local-first inference; hardware-tier packs can slot into provider list |
| Plugin lifecycle hooks for policy enforcement/auditing | 🧩 Mechanism only | Kernel hook points § | Hook architecture pattern for allow-list enforcement, not a dependency |
| Coordinator→specialist multi-agent teams | 🧩 Mechanism only | Orchestrator § | Routing shape is kernel-mediated hierarchy; adopt topology only |
| Kanban board w/ per-card worktree, auto-commit, dependency chains | 🧩 Mechanism only | Task queue UI § | Worktree-per-task isolation is a good pattern; board itself implies a standing service |
| Scheduled cron agents persisting across restarts | ❌ Conflict-skip | — | Standing background process violates ephemeral-worker invariant |
| Messaging connectors (Telegram/Slack/Discord/WhatsApp/Linear) | ❌ Conflict-skip | — | Standing third-party inboxes violate ephemeral workers + default-deny egress |
| Background process watching/reacting | 🧩 Mechanism only | Worker supervision § | Output-reactive supervision useful inside bounded worker lifetime |

## Notable absences
No deny-by-default filesystem/network allow-lists beyond human approval (auto-approve is all-or-nothing). No immutable structured audit log — history is session transcript-based. No department/memory gating concept. No non-technical onboarding surface (CLI/npm-first). Team/schedule persistence requires a running host, which LastHarness's one-box model must replace with kernel-spawned ephemeral runs.
