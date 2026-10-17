# LastHarness Roadmap

A department-led, local-first, endpoint-agnostic AI harness for power users, with a memory that lasts. The decisions behind this plan are in `.grill/lastharness-reboot.md`. What the product is lives in `spec/lastharness.md`; this file says when each part is built. The old design is archived in `archive/lastharness-v1-spec.md` for reference only.

**Status:** draft. Nothing is built.
**Stack:** Rust core with a local API. Front ends: terminal, browser, then native GUI.
**Estimates:** build weeks for one developer working with an AI assistant, at full focus, accurate to about ±50%. Scale them to your real hours.

---

## Timeline

| # | Milestone | Weeks | Cumulative | Gate to pass |
|---|---|---|---|---|
| 0 | Spikes | 1 | 1 | Questions below answered with numbers |
| 1 | Core and terminal chat | 3 | 4 | Real conversation over three different servers |
| 2 | Memory v1 | 3 | 7 | Remembers across restarts; sensitive data never leaves |
| 3 | Learning v1 | 3 | 10 | A repeated habit becomes an approved rule |
| 4 | Roles and planning | 3 | 13 | A big goal runs plan → approve → milestones |
| 5 | Onboarding and settings | 2 | 15 | New user reaches a working setup without docs |
| 6 | Browser UI | 3 | 18 | Everything in the terminal works in the browser |
| 7 | Messaging | 3 | 21 | Chat from a phone through a messaging channel |
| 8 | Native GUI | 5 | 26 | Passes RAM and screen-reader checks |
| 9 | Mobile app | 7 | 33 | Pair a phone, chat, approve, and wake an idle core |
| 10 | macOS and Windows | 5 | 38 | Same test suite passes on all three systems |
| 11 | Release | 2 | 40 | Single install per platform, docs, security pass |

Milestone 0 may take one to two weeks, to be decided when it starts; the totals assume one.

**About 40 weeks end to end**, not counting the hosted relay and subscription (see Track S below). Development is Linux-only and private until the first public release, which ships on Linux, macOS and Windows together. A usable terminal product exists on Linux at about week 10, which is the first point worth showing to anyone you trust.

From Milestone 1, keep OS-specific code (paths, shell, key storage, process control) behind one thin layer and run the test suite on all three systems in CI, so Milestone 10 is a port and not a rewrite.

The weeks above have not yet been adjusted for web search, the activity log, auto-compaction, projects and assistant mode, MCP, and memory sync. Treat the total as a lower bound.

---

## Milestone 0: Spikes (weeks 1–2)

Throwaway code that answers the questions the design depends on.

- [ ] Stream chat and tool calls against three servers (your llama-swap, Ollama or LM Studio, one cloud API). Record every place their behaviour differs.
- [ ] Measure a minimal Rust terminal app's RAM (PSS, not RSS) idle and in a long session.
- [x] Prototype memory retrieval two ways (keyword only; keyword plus a small built-in embedding model) on real conversation text in English and Arabic, and compare. Candidate models: EmbeddingGemma 2 (text-only 270M configuration; Apache 2.0), EmbeddingGemma 1, BGE-M3, multilingual-e5-small. Measure RAM, load time and query time, and check each model's licence.
- [x] Decide the local store: SQLite.
- [ ] Measure end-of-session extraction cost on a local model: cold prefill vs prefix-cache hit vs slot save/restore (llama.cpp), on this machine.

**Exit:** a short written answer to each, with numbers.

## Milestone 1: Core and terminal chat (weeks 2–4)

- [ ] Provider layer: base URL, key, model list, capability probing (tools, thinking, context size).
- [ ] Compatibility table per provider and per model (auto-detected, overridable): reasoning format, developer role, token-limit field, tool-call quirks. Tolerant parsing of partial streaming JSON. Native tool calls only, as Pi, Codex and OpenCode do; no prompted fallback.
- [ ] Streaming chat, cancel, retries, clear errors.
- [ ] Image and video generation through providers (OpenRouter and other cloud, plus local endpoints). On or off in settings.
- [ ] Spend tracking per provider and per request, with three selectable modes: no limits, hard caps, estimate-and-approve with thresholds.
- [ ] Secret storage for keys; never plaintext in config.
- [ ] Local API so any front end can drive the core.
- [ ] Core lifecycle: always on, starts at login, survives front ends closing (collapses to the tray when there is one), and enters a near-zero idle mode after a configurable time. Idle means no timers or polling, event-driven wake-up. Measure idle CPU wake-ups and RAM.
- [ ] In idle only a minimal authentication listener stays open. Paired devices (keys exchanged once, for example by QR code) authenticate automatically; a success wakes the core; unknown devices are rejected before any data is parsed.
- [ ] Auto-compaction when a conversation nears the model's context limit; the context length it starts at is adjustable in settings. Summaries are appended so the prefix cache is kept.
- [ ] One append-only activity log: every tool call, every request that left the machine, every rule, skill, role or setting change, with undo where it makes sense.
- [ ] Web fetch and web search tools. Search provider chosen at setup: none, a paid search API with your key, the subscription's search, an existing SearXNG, an app-managed SearXNG (setup shows its RAM cost), or a free provider such as TinyFish.
- [ ] Projects are folders: opening a folder attaches its memory. Folderless mode: plain chat, or a general assistant starting in the home folder (it uses the same permissions as everywhere).
- [ ] Terminal UI.
- [ ] One role runner with a tool loop and deny-by-default tools.
- [ ] `sudo` is off for agents by default; onboarding offers to enable it.
- [ ] File and command tools with tiered permissions (default), ask-for-everything mode, per-command allow rules, and a `--dangerously-skip-permissions` bypass with a risk warning. A setting picks what the bypass covers (everything, tools only, or tools plus hard locks; default the last).
- [ ] Per-role sampling and template settings.

**Exit:** a real task finished on a local model and on a cloud model, same session format.

## Milestone 2: Memory v1 (weeks 5–7)

- [ ] Four layers: preferences, facts about you, projects and decisions, past conversations.
- [ ] Every item, rule, session, setting and log entry has an owner and a scope (personal, project, team). One owner in the first release; multi-user comes after launch.
- [ ] Local storage and hybrid retrieval: keyword always on, semantic search with a small built-in multilingual embedding model (CPU, loaded on demand, unloaded in idle), results shown progressively; a bounded amount of memory per request.
- [ ] Privacy classes. Default sensitive categories detected by patterns plus model judgment; uncertain items count as sensitive. Per-provider trust levels (local only, own cloud account, any cloud). The user can reclassify items and see exactly what was sent.
- [ ] End-of-session review: the harness proposes facts and decisions, you accept, edit or drop. Prompts are built append-only so the prefix cache is reused.
- [ ] "Remember this" and "forget this" by asking the assistant; incognito (temporary) sessions that store and learn nothing.
- [ ] When a fact changes, the old one is archived: left out of normal recall, searchable, restorable.
- [ ] Memory viewer: search, edit, delete.
- [ ] Memory as files: one Markdown file per page with a header, links between pages, an index file per layer. Hand edits are applied when detected, the old version archived and logged. Conversations stay in the database and export to Markdown. Export and import as folders.
- [ ] An "about you" block, always included, assembled from one-line gists written once per accepted page (in the review turn) and picked by pins and relevance up to a size cap. Never regenerated as a whole; never overwrites your edits.
- [ ] Maintenance pass (contradictions, duplicates, stale facts, orphan pages) that only proposes changes. On demand by default; a schedule for idle time is a setting.
- [ ] Append-only history of memory changes.

**Exit:** restart and it remembers; a request to a cloud model provably excludes sensitive memory.

## Milestone 3: Learning v1 (weeks 8–10)

- [ ] Detect repeated corrections and habits, with the evidence attached.
- [ ] Rule proposals you accept, edit or reject. Stated rules ("always do X") also go through confirmation.
- [ ] Style profile: one short editable document for tone and manner, applied to every role, learned and changed through the same proposals as rules.
- [ ] Skill proposals: after a complex task, the harness offers a reusable procedure (Markdown file) you accept, edit or reject. Skills can be improved with use, as proposals. Check compatibility with the agentskills.io format.
- [ ] Rules apply to the right context and the most specific one wins.
- [ ] Snapshots and rollback; the base prompt is never rewritten.
- [ ] Setting for silent learning (off by default).

**Exit:** a habit seen several times becomes a visible, editable, revertible rule.

## Milestone 4: Roles and planning (weeks 11–13)

- [ ] Role definitions: instructions, tools, model type or specific model, sampling, memory slice.
- [ ] Model types: stock frontier, mid-tier and fastest; custom types (for example Coder, Researcher); add, edit and delete any type including stock ones, with reassignment pop-ups for affected roles before a delete.
- [ ] When a type's model is unavailable: setting for stop with a clear message and a button to pick another model now (default), or an ordered fallback list per type.
- [ ] Workers run concurrently where the endpoint or hardware allows (own model each, or one shared model), and queue otherwise. Limit is a setting.
- [ ] Departments: a group of workers with a lead that breaks a goal into steps and assigns them; nestable; stored as a Markdown file.
- [ ] Project leader: the assistant recognises a project request and spawns a clone of itself (same tools and permissions, cannot spawn another leader, confined to the project folder) that calls the departments.
- [ ] Starting a project: clear request starts with a one-line notice; a location in the prompt is used; unsure means ask; away mode never asks.
- [ ] Departments can be called directly, outside a project (assistant is the caller).
- [ ] Away mode: assistant decides on its own within the user's permission rules; never approves sensitive categories; sudo only when available and needed; decisions logged and summarised on return; undecidable parts parked.
- [ ] Failures: a failed worker reports to its caller, who resolves it (do it itself, spawn a regular subagent, or hand off); unresolved failures go up the chain. Caller can also change the model. Top-level reports to the user. Attempt cap per step (default 3).
- [ ] Communication: lead briefs workers, workers return result plus summary, no worker-to-worker messages.
- [ ] Quality checks: lead checks summaries; small mistakes are fixed in place (lead or a fresh instance of the same worker), larger misses follow the failure path.
- [ ] Terminal: per-worker list (model, step, status) with open, pause and stop. The flow diagram comes with the GUIs.
- [ ] Planning: propose breakdown, you approve or edit, then run.
- [ ] Autonomy settings: plan-first, milestone check-ins, fire-and-forget.
- [ ] Agents can select models and operate them (load, unload, settings) within registered providers.
- [ ] Agents can propose new providers and keys; you approve.
- [ ] MCP client: use MCP servers (for example Higgsfield's) as tools. Adding a server needs approval; its tools go through the permission tiers.
- [ ] Skills in the open Agent Skills format (contents preview, scripts listed). MCP servers by link (remote) or command (local, exact command shown), tools first. Folder trust pop-up has a button to disable the folder's skills and MCP servers and use your own.
- [ ] Themes: select custom themes (for example liquid glass) and download community themes. On Omarchy, everything follows the Omarchy theme (template in `~/.config/omarchy/themed/`, hook in `theme-set.d`, translucency), live. Own theme format (colours, opacity, blur, etc.; field list to be written). Blur on Omarchy comes from Hyprland's settings.
- [ ] Permission requests go up the chain (sudo, outside the folder): each level accepts, rejects or passes up; the top is the assistant, then the user, or away mode.
- [ ] Parallelism is adjustable: sequential, queued, or N at a time.
- [ ] Plugins as data bundles (workers, departments, themes; skills and MCP servers are separate), installed from a folder or link, with a contents preview and project trust. No code extensions in the first release (see spec §9.2).

**Exit:** a multi-part goal completes with plan approval and milestone pauses.

## Milestone 5: Onboarding and settings (weeks 14–15)

- [ ] First run in the terminal: connect a provider, pick memory and learning behaviour, autonomy level, media on/off, spending mode, web search provider, and opt-in crash reports (with preview). How these are presented (profiles, step-by-step, or just-in-time) is decided last.
- [ ] Single settings surface for everything above.
- [ ] Export and import of memory, rules and roles.

**Exit:** someone who has used LM Studio gets to a working, remembered setup unaided.

## Milestone 6: Browser UI (weeks 16–18)

- [ ] Served by the core on localhost, no extra app.
- [ ] Responsive layout, usable from a phone browser too (over the local network or the user's VPN, behind the same pairing and authentication).
- [ ] Core serves the UI over HTTPS with its own certificate; the pairing PIN confirms the fingerprint; the browser holds a non-exportable key; paired devices listed with revoke.
- [ ] Chat, memory viewer, rule review, plan approval, settings.
- [ ] Accessible by default.

## Milestone 7: Messaging (weeks 19–21)

- [ ] Channel interface in the core.
- [ ] Every channel is opt-in and off by default.
- [ ] Channels: Telegram, Matrix, Signal and Slack. Channels with privacy concerns (Slack) show a clear warning at connection. Slack: each user creates their own app, which avoids the Marketplace rate limits.
- [ ] WhatsApp is out for good: no channel, no skill, no support.
- [ ] Approvals work from the phone.
- [ ] Channels follow the same sensitive-data rule as cloud models, with a low trust level and a warning that they carry no security promise.
- [ ] An enabled channel wakes the idle core when an allow-listed sender messages it. Messages from anyone else are ignored without waking anything.

## Milestone 8: Native GUI (weeks 22–26)

- [x] Week 1: toolkit test (Slint, egui, iced, GPUI, Qt Quick): `spikes/gui-toolkits/RESULT.md`. The user chose GPUI. Web views and closed or paid engines were rejected.
- [x] The GUI on the same local API: `crates/lh-gui` (see `docs/milestone-8-report.md`): three-part window, chats with streaming, stop, model per chat, stored chats and resume with a warning, departments, memory, settings, opened content with pop-out, eight themes with live reload, right-to-left mirroring, keyboard shortcuts, tray, screen-reader tree.
- [x] Target: noticeably lighter than an Electron app: about 115 MB idle, 0.0 to 0.4% CPU idle, 64 MiB binary.
- [ ] Left for later: automatic opening of agent content, the KV cache per chat, Orca check with real speech, drawing the app's own window frame, Servo as an optional add-on.

## Harness milestone (before mobile)

Approved by the user before mobile (decision log, "harness milestone"). Built while the user was away on 2026-10-17. The working choices and open questions are in `.grill/parked.md`.

- [x] File checkpoints and undo of a whole turn (`/v1/sessions/{id}/undo_turn`, a palette entry in the app).
- [x] Diff review: a file write's permission question shows what changes.
- [x] Command sandbox under the permission tiers (`tools.sandbox`, bubblewrap).
- [x] Hooks before and after tool calls (`hooks.toml`).
- [x] Headless `lh -p`.
- [x] Scheduled tasks (`lh schedule`, `/v1/schedules`). A page in the app is not built yet.
- [x] Code intelligence for coding roles: `code_lookup` over LSP.
- [ ] Servo replacing Blitz in the same sandbox: helper written (`crates/lh-pageview-servo`), not built or tested yet.
- [ ] Voice comes after, optional.

## Milestone 9: Mobile app (weeks 27–33)

- [ ] Week 1 spike: verify Keychain and Keystore key storage, push notifications, and Arabic and right-to-left text before committing.
- [ ] One Rust client library shared by both phones (pairing, key handling, protocol), exposed through UniFFI. Native thin UIs: SwiftUI on iOS, Jetpack Compose on Android.
- [ ] Pairing by QR code; the device key is kept in the phone's secure storage.
- [ ] Automatic authentication of the paired device; success wakes an idle core.
- [ ] Chat, plan approval, rule and memory review, notifications.
- [ ] Direct connection (same network or the user's own VPN) and the relay fallback.
- [ ] Estimate is a rough guess (6–8 weeks); revisit after the browser UI exists.

## Milestone 10: macOS and Windows (weeks 34–38)

- [ ] Ports of the platform layer: paths, shell and command tools, secret storage, process control.
- [ ] Permission tiers and the command tools working on each system (Unix-style commands behave differently on Windows).
- [ ] Native GUI and screen-reader checks on each system (Orca on Linux, VoiceOver on macOS, Narrator or NVDA on Windows).
- [ ] CI runs the full suite on all three.

## Milestone 11: Release (weeks 39–40)

- [ ] Single installer per platform.
- [ ] Signed updates, on by default and switchable off in settings. No usage analytics.
- [ ] Documentation and examples.
- [ ] Security review of key storage, tool permissions and the data-leaving rule.
- [ ] Open source release: the harness, plugin format and apps under Apache-2.0, the relay server under AGPL-3.0 in a separate repository; publish the repositories.

---

## Track S: Subscription (starts after the first release)

Separate repository and timeline, no dates yet. The first release works fully without it, and the settings show its features as "coming later" until it exists.

- [ ] Accounts and login.
- [ ] Billing.
- [ ] Hosted relay (AGPL-3.0, in its own repository; self-hosting stays free).
- [ ] Hosted web search.
- [ ] Memory sync across devices, end-to-end encrypted (key derived from the account password, with a recovery code).
- [ ] Support.

---

## Not planned

- WhatsApp, ever: the unofficial route risks bans and the official one needs a public webhook.
- Controlling other agents (Claude Code, Codex, Pi, Hermes).
- Installer, hardware detection, model packs, benchmark sweep.
- Editor integration.
- Electron.

## Later (not in the first release)

- Code extensions, Pi-style (the user said "not yet", not "never"). Until then, code that needs to run comes in as an MCP server.

## Open decisions

1. Memory model and store choice (Milestone 0).
2. Onboarding presentation: profiles, step-by-step, or just-in-time questions.
3. Name: keep LastHarness. Before release, check the domain, package names and trademark (Harness is also a DevOps company).
4. Subscription: decided as Track S, after the first release (see above).

