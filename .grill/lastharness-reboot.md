# Grill: LastHarness reboot
Date: 2026-10-08

## Intent
A harness for tech-literate power users (Linux users, Office-suite power users, people who already use ChatGPT, DeepSeek or LM Studio). Its core value is a **memory that lasts** and an AI that **learns how you work**, so you stop re-explaining yourself. It should also be the most compatible and versatile harness: any model endpoint, several ways to talk to it.

## Constraints
- Own harness only. Not a control plane for other agents (Zeron / MonoCode territory).
- No Electron. RAM is scarce; models need it first.
- Memory is local. Cloud models see only what is relevant to the task; sensitive categories are never sent.
- Approval by default for anything that changes behaviour or sends data somewhere new. Relaxable in settings.
- Dev machine: i5-11400H, 24 GB RAM, RTX 3050 4 GB. The user's earlier benchmark results are not reliable (sampling and templates were unfinished); do not treat them as quality evidence.

## Key decisions
- Decision: memory covers all four layers: working preferences, facts about the user, projects and decisions, past conversations. Reason: user wants all of it. Alternative considered: skip personal facts at first.
- Decision: learning follows Prime Agent's Continual Harness idea (evidence-backed updates to editable supplemental state, base prompt never rewritten, rollback). Every rule, stated or inferred, is a proposal the user confirms by default. Silent learning is an opt-in setting. Reason: avoid silent misbehaviour.
- Decision: autonomy levels (plan-first, milestone check-ins, fire-and-forget) and learning mode are settings chosen at onboarding with safe defaults.
- Decision: workers can run concurrently, each with its own model or sharing one. The scheduler parallelises when the endpoint/hardware allows and queues otherwise; the limit is a setting. (Corrected: earlier "sequential only" was based on the dev rig alone, and the product is not just for that rig.)
- Decision: LastHarness sits on top of any endpoint: OpenAI-compatible, Anthropic-style, Lemonade, Ollama, llama.cpp, LM Studio, custom, any cloud provider. It does not manage inference (no installer, model packs or benchmark sweep).
- Decision: agents may select models, operate them (load, unload, settings) and register providers. Registering a provider or key needs owner approval by default.
- Decision: front ends are terminal, native GUI, browser UI, and messaging (dedicated app plus messaging apps). Every messaging channel is opt-in; channels with privacy concerns (Slack) show a warning when connected. No editor integration (MCP is supported; see the extensions decision).
- Decision: core in Rust. Terminal UI first, browser UI second, native GUI last after testing toolkits (Slint, egui, iced, GPUI) for RAM and accessibility. Tauri-style wrapper is the fallback.

- Decision: image and video generation are in from day one, via providers (OpenRouter and other cloud, plus local endpoints). Toggle at onboarding and in settings. Reason: user wants versatility; an off switch removes the risk.
- Decision: spending control has three modes (no limits, hard caps, estimate-and-approve with thresholds), all selectable, chosen at onboarding and changeable later. Reason: agents can call paid APIs.
- Decision: every behaviour choice is a setting, set at onboarding with a safe default. How onboarding presents them is deferred to last.

- Decision: rule proposals are triggered by repetition across sessions (threshold N, default 3, user-settable) and by explicit signals (corrections, "always/never", thumbs). The model confirms the pattern is real and writes the rule text and evidence; the user then approves. Reason: traceable, cheap, and works with a small local model. Alternative considered: model-judged patterns only (noisy, costs tokens every session).

- Decision: tool permissions are tiered by default (reads automatic; writes in the project ask once per session; outside the project, sudo, delete, install and network always ask). Per-command allow rules can be promoted, and promotion is a confirmed rule. "Ask for everything" is a selectable setting. A `--dangerously-skip-permissions` flag exists for full bypass. No sandbox is planned (not requested). Reason: matches Claude Code / Codex / OpenCode permission models.

- Decision: what the bypass flag covers is a setting with three levels: everything off; tool permissions only; tool permissions plus hard locks (sensitive-data rule, silent key registration). Default: the third. Using the bypass shows a clear warning that the user accepts the risk and the project is not responsible for the outcome. Reason: users differ on risk; the warning makes the choice explicit.

- Decision: roles are plain Markdown files (header: name, model, tools, sampling, memory slice; body: instructions) that are the source of truth, and the app edits them through a form or chat. Role changes proposed by the harness go through the same confirm-and-rollback path as rules. A starter set ships (researcher, writer, coder, designer, planner). The planner groups steps by model to limit load time (about 14 s on the dev rig).
- Decision: models are assigned to roles through model types. Three stock types ship: frontier (the best model available), mid-tier and fastest, like Opus, Sonnet and Haiku. The user can assign a role to a type or directly to a specific model, create custom types (for example Coder → Ornith 1.5 35B A3B, Researcher → Apodex 1.1 mini), and add, edit or delete any type, including the stock ones. Deleting a type that roles use shows a warning and pop-ups to reassign each affected role before the deletion goes through. Reason: user's design; a role then follows the type when the underlying model changes. Reason: audience edits files, newcomers need a form.

- Decision: memory capture is auto-propose then review. After a session the harness proposes facts and decisions; the user accepts, edits or drops them. Raw conversations are always stored and searchable but never promoted to facts without approval. "Remember this" works any time. A setting can enable auto-accept later. Reason: same confirm-by-default principle as rules.
- Decision: the end-of-session extraction must not re-read the whole conversation on slow local prefill. Approach: build prompts append-only so the conversation prefix stays byte-identical and the server's prefix cache is reused; ask the extraction questions as a final turn; use server slot save/restore (llama.cpp) where the capability probe finds it; fall back to incremental extraction or running when idle. Reason: user's local models have slow prefill.
- Decision: forgetting is done by asking the assistant ("forget X", with a visible list of what was removed) or by running an incognito session, a temporary session that stores and learns nothing. Reason: user's choice; mirrors ChatGPT/Claude temporary chats.

- Decision: add skill creation modelled on Hermes. After a complex task the harness proposes a reusable skill (a Markdown procedure for a type of task) with the evidence attached; improvements to a skill during use are also proposals. Unlike Hermes, which grades itself with no gate, every skill is confirmed by the user, versioned and revertible. Skills are files like roles (agentskills.io compatibility to be checked). Reason: user wants it; fits the existing proposal-and-review path.

- Decision: sensitivity is decided by default categories (credentials, health, finance, identity documents, anything marked private), found with simple pattern rules plus model judgment when an item is saved, combined with per-provider trust levels (local only, own cloud account, any cloud). An item goes only to a provider whose level is high enough. Uncertain items count as sensitive and appear in the review pass. The user can reclassify any item, and a "show me what was sent" view is the backstop. Reason: protection without remembering to tag; "cloud" is too blunt a line.

- Decision: endpoint quirks are handled the way Pi, Codex and OpenCode do it. Use the server's native tool calls, with a compatibility table per provider and per model (auto-detected from URL and provider, overridable per model), several reasoning-field names accepted, and tolerant repair of partial streaming JSON. No prompted tool-call fallback. Reason: Pi works across MiniCPM, LFM, Qwen and Ornith on the user's llama-swap this way; the server's chat template does most of the work.

- Decision: fully open source. Reason: the privacy promise (sensitive data never leaves) is only believable if people can read the code. Licence choice (MIT vs Apache-2.0) still to make. Alternative considered: closed source. Amended twice: a paid LastHarness subscription covers three optional services: the hosted relay, hosted web search, and memory sync across devices through a login. The harness itself stays free and fully usable without an account. The relay code is open and self-hostable. Reason: keeps the privacy promise checkable, and the subscription is a convenience, not a lock-in.
- Decision: the mobile app reaches the machine both directly (same network or a mesh VPN the user runs) and through a relay that passes only encrypted data. The hosted relay comes with a LastHarness subscription. Direct connection is tried first. Reason: user's choice; works away from home without network setup for subscribers.

- Decision: the first public release ships on Linux, macOS and Windows together. Until then development is Linux-only and private. Reason: the audience is not only Linux users. Cost: about 5 extra weeks (roadmap now 33 weeks) and a rule from Milestone 1 to keep OS-specific code behind one thin layer with CI on all three systems.

- Decision: a model type can point at any model the user has available, cloud or local, with no restriction. When a type's model is unavailable (outage, not loaded, spend cap hit, or not allowed for sensitive memory) the behaviour is a setting with two modes: stop with a clear message that has a button to pick another model now (default), or fall back along an ordered per-type list (never to a model the sensitive-data rule forbids; the user is told). "Ask each time" is not a separate mode; it is the button. Reason: user's choice.

- Decision: the core is always on. Closing a front end collapses it to the system tray; the core keeps running. After a user-set idle time it enters an idle mode that uses almost no resources (the way Spotifast does: it stopped keeping the audio device busy while paused, taking idle CPU to roughly zero). Reason: messaging, long plans and the end-of-session review need a running process, but RAM goes to models first.

- Decision: a mobile version of the app is planned. In idle mode only an authentication listener stays open. A paired, verified device authenticates automatically (no password each time) and a successful authentication wakes the core. Reason: user's design; actions from a verified device are trusted. The mobile app is a native thin client for Android and iOS built on one shared client codebase, in the first release (roughly 6–8 weeks, rough guess). The browser UI is also responsive and usable from a phone browser over the local network or VPN, behind the same pairing. Reason: the device key needs the phone's secure storage, which a PWA cannot match.

- Decision: messaging channels (Telegram, Matrix, Signal, Slack) are convenience channels with no security promise from LastHarness. Each channel gets a low provider-style trust level, so sensitive memory never goes over it and the user sees a clear warning. The secure path is the paired mobile app. No home-made cryptography. Reason: user's stance; the existing sensitive-data rule already covers it.

- Decision: an enabled messaging channel keeps its quiet outbound connection open while the core is idle, and a message from an allow-listed sender wakes the core. No per-channel keep-awake toggle; disable a channel to stop it. Relay-held channel connections are not planned. Reason: costs almost no CPU, needs nothing from the relay, and the relay never holds the user's bot tokens.

- Decision: the first release's channels are Telegram, Matrix, Signal and Slack. WhatsApp is out for good: no channel, no skill, no support (the unofficial route risks bans, the official one needs a public webhook). A WhatsApp CLI such as wacli was considered as an agent tool and dropped. Reason: user's call.

- Decision: the old spec is archived in `docs/archive/` and a new spec (`docs/spec/lastharness.md`, v0.1) is written from the decisions in this log, in the same document style as the old one. The roadmap says when; the spec says what. Reason: user wants a spec like the original.
- Decision: device pairing works like Sunshine and Moonlight. The new device (phone or browser) shows a PIN, the user enters it on the machine, keys are exchanged, and from then on the device authenticates automatically by its key. A browser holds its key less safely than a native app, and the settings say so. Reason: user's instruction.

- Decision: keep the name LastHarness ("the last harness you'll need"). A quick search found no existing software with that name. Before release, check the domain, package names and trademark, since "Harness" is also the name of a DevOps company. Reason: the name carries the product promise and no better candidate was found.

- Decision: tone and style are kept in a single style profile, a short editable document describing how the user likes to be talked to (terse or chatty, how much to explain, whether to ask before acting). It applies to every role. Task habits stay as rules. Both use the same propose-and-confirm path, with snapshots and rollback. Reason: tone applies everywhere, so one profile is easier to see and fix than scattered rules.

- Decision: when a conversation nears the model's context limit, the harness auto-compacts it. Summaries are appended, so the prefix cache is not broken. The context length at which compaction starts is an adjustable setting. Reason: user's choice.
- Decision: one append-only local activity log records every tool call, every request that left the machine, and every rule, skill, role or setting change, with undo where it makes sense. Reason: extends the "show me what was sent" view.
- Decision: a project is a folder. Opening the harness in a folder attaches that project's memory and decisions, and global memory applies everywhere. There is also a folderless mode: plain chat, or a general assistant that starts in the user's home folder and does tasks for them. Reason: user's choice.
- Decision: web search is presented at setup as an essential choice, not hidden behind a default of "none". Options: none, a paid search API with the user's key, the subscription's search, an existing SearXNG instance, an app-managed SearXNG instance (setup shows its RAM cost), or a free provider such as TinyFish (free Search and Fetch API).
- Decision: permissions work the same in every mode, including the general assistant that starts in the home folder: a Claude Code-style default, the ask-for-everything setting, and the bypass mode. There are no special assistant-mode rules. `sudo` is off for agents by default, and onboarding offers to enable it. When enabled, `sudo` asks like any other command and can be covered by Claude Code-style allow rules. In bypass mode `sudo` does not ask: bypass is bypass. Bypass removes prompts but does not turn features on: if `sudo` is off in settings it stays off in bypass mode. Reason: user's choice; one system is easier to understand.
- Decision: the first release has one owner per install, but every memory item, rule, session, setting and log entry carries an owner and a scope (personal, project, team) from day one. Multi-user mode comes after launch, modelled on Microsoft Teams: Owner, Member and Guest roles, personal and project memory kept apart, requests built only from the asker's memory and projects, and the host owner able to see everything unless per-person encryption is added later. Reason: user wants multi-user; the data model is the cheap part to get right now.
- Decision: the memory architecture is a remix. Layers: a short always-on "about you" block, assembled from one-line gists written once per accepted page in the review turn and never regenerated as a whole (regenerating is slow and costly on local models); Markdown pages (preferences, facts, projects and decisions) with links and an index file per layer; raw conversations in the database, never edited and exportable; and behaviour (rules, style profile, skills) kept apart. Changed facts are archived. Hand edits to files are applied when detected, with the old version archived and logged. The maintenance pass (contradictions, duplicates, stale facts) runs on demand by default, with an idle schedule as a setting; its output and the gists are proposals that never overwrite the user's edits. Inputs: Claude's memory, TencentDB Agent Memory, Karpathy's LLM wiki, Mem0, Claude-Mem, Obsidian. Reason: no single design covers it, and the user wanted a remix.
- Decision: licences. The harness, plugin format and apps are Apache-2.0 (permissive, patent grant, friendly to plugins and the iOS app). The relay server is AGPL-3.0 in a separate repository, so a competitor cannot run a closed hosted fork of the paid service; self-hosting stays free. Alternatives considered: MIT, GPL for everything. Reason: user's choice.
- Decision: the mobile apps share one Rust client library (pairing, key handling, protocol) through UniFFI, with native thin UIs: SwiftUI on iOS and Jetpack Compose on Android. Bitwarden, Signal and Firefox use the same shape (shared Rust core, native UIs). A week-1 spike verifies Keychain and Keystore key storage, push notifications and Arabic text. Alternatives considered: Flutter, React Native, Tauri 2 mobile. Reason: identity-and-protocol apps in this category share a systems-language core.
- Decision: plugins are anything that extends the app. In the first release they are data bundles: workers, departments, skills, MCP server entries and themes, installed from a folder or a link, with a contents preview and project trust. Pi-style code extensions are undecided and parked. Reason: user confirmed data bundles and could not yet decide on code extensions.
- Decision: memory search is hybrid. Keyword search is always on; semantic search uses a small multilingual embedding model shipped with the core (CPU, loaded on demand, unloaded in idle), chosen in Milestone 0 by testing English and Arabic, EmbeddingGemma 2 text-only first. The user may point it at their own embedding endpoint; embeddings never go through a cloud endpoint unless allowed. Results are shown progressively. Reason: user wants semantic matches ("sadness" finds "cry"), and the embedder is small enough to bundle.
- Decision: positioning. The product leads with departments (a team of specialist workers you hand a goal to) and is local-first (models and data on the user's machine by default; cloud optional). Memory is what each worker runs on. Reason: user's choice; departments are the part that most interests them. The suggested Milestone 0 comparison (single generalist versus a department on the same task) was not adopted or rejected.
- Decision: the user can export and import memory, edit it as files, or tell the agent to edit it. The details are parked (see open questions). Reason: user's choice; memory should never be locked away from its owner.
- Decision: the harness supports plugins and MCP, which reverses the earlier "no MCP integration". The model case is hosted MCP servers such as Higgsfield's, which give an agent image and video generation through one endpoint. Editor integration stays out. MCP is client-only: the harness uses MCP servers as tools and is not itself an MCP server. MCP servers follow the existing rules and get no separate trust system: adding one needs approval (like a provider), its tools go through the permission tiers, and the sensitive-data rule applies to whatever the agent sends. Agents send prompts and arguments, never a memory dump. Plugins can be a single worker (a role), a whole department (a group of roles, optionally with sub-departments), or a bundle of skills and MCP entries, in the manner of Pi packages. Plugins work like Pi packages: a package bundles executable extensions (code), skills, prompts, themes, workers and departments, and is installed from npm, a git link or a local path. Extensions run with the user's full permissions, so the app warns clearly and the user installs only from sources they trust. A folder's own plugins and MCP servers load only after the user trusts that folder (Pi's project trust). How extension code runs inside a Rust core is not yet decided.
- Decision: the interface is English first. Translations, right-to-left layout and Arabic are planned but not committed; the user will try them with agents. Right-to-left and Arabic text shaping become test criteria for the native GUI toolkit choice.
- Decision: all five Milestone 0 spikes stay. Their length stays flexible (one to two weeks, decided when they start). Reason: the plan is still liquid.
- Decision: memory sync works with a login and no separate key. The encryption key is derived from the account password, so the servers hold only ciphertext, and a recovery code is offered at signup. Forgetting both loses only the synced copy, not the PC copy. Reason: easy for the user without letting the servers read memory.
- Decision: when a fact changes, the old fact is archived, not overwritten or deleted. Archived facts are left out of normal recall, stay searchable and can be restored. The user can still delete anything on request. Reason: user's preference (archive).
- Decision: updates are on by default (signed) and can be turned off in settings. Anonymous crash and bug reports are opt-in, asked at onboarding, with a preview of exactly what would be sent and a setting to send automatically. No usage analytics.
- Decision: memory is a system, not a single file: a database with the four layers as separate stores, each item having a type, source, privacy class and timestamps, plus an append-only history, and a readable Markdown export per layer. Design inputs to study: TencentDB Agent Memory, Claude-Mem, Mem0, Obsidian-style second brains, and Google's TurboQuant (compressed vector indexes, only worth it if embeddings are used and the index gets large).

## Surfaced assumptions
- The department/kernel structure from the old spec is not required. Roles replace it.
- The spec's "kernel always local" rule is replaced by the memory privacy rule above.
- Rust's slower iteration is acceptable; AI-assisted development offsets it.

## Open questions
- Positioning is decided (see decisions).
- Plugins: no code extensions of any kind yet, not even Pi-style (user decision). Code that must run comes in as an MCP server.
- Storage and retrieval design for memory (local embeddings or not).
- Tool-call normalization across servers with different behaviour.
- Onboarding presentation: profiles, step-by-step, or just-in-time (decide last).

## Out of scope
- Controlling other agents (Claude Code, Codex, Pi, Hermes).
- Installer, hardware detection, model packs, benchmark sweep.
- Editor integration.
- Electron.

- Decision: a department is a named group of workers with a lead; the lead breaks a goal into steps and assigns them to members, independent steps run concurrently; departments nest; defined as a Markdown file; the approved plan and autonomy level sit on top.

- Decision: while a department works the GUI shows a live flow diagram (goal -> lead -> worker nodes tagged with model, glowing while active, results flowing back), with a plain per-worker list for the terminal and screen readers. Inspiration: OmniRoute's flowing provider animation. Details parked until the user shows the animation.

- Decision: a failed or stuck worker reports to whoever called it, and the caller resolves it. A lead can do the step itself, spawn a regular subagent, or hand it to another worker. Failures that a department cannot resolve go to whoever called the department. (User corrected my proposal of retry-then-pause-with-buttons.)
- Decision: the caller can also change the model and retry. If a failure reaches the top unresolved, the top-level assistant reports what failed and what was tried, and waits. Each step has an attempt cap (default 3, a setting) covering retries and spawned subagents.

- Decision: the lead briefs each worker (step, needed inputs, allowed memory slice); workers return a result plus a short summary; the lead reads summaries and can open transcripts; workers never message each other.

- Decision: the lead checks each summary against the step. Small mistakes are fixed in place by the lead or by a worker (the same worker as a fresh empty-context instance), with no redo. Larger misses count as failures and follow the failure path. Checker model is a role setting.

- Decision (user's most important): the regular assistant auto-recognises a project-creation request and spawns a project leader, a clone of itself with the same tools and permissions, but it cannot spawn another project leader and cannot act outside a set project folder. The project leader is the CEO: it calls the departments and handles making the project.
- Decision: the folder limit is a hard lock (no prompt). Needs from outside go from the leader to the assistant, which asks the user, or acts on its own when an away mode (human unavailable) is set.
- Decision: away mode follows the user's permission rules and allow rules; the assistant decides what they would ask; never approves sensitive categories or sends sensitive data to cloud or channels; sudo is not banned, it follows the sudo setting and is used only when needed, with caution (user corrected my 'never enables sudo'); every decision is logged and summarised on return; undecidable parts are parked while the rest carries on.
- Decision: the user can ask for a department directly; it runs in the current folder with the assistant as caller and the usual permissions; the project leader is only for project creation.
- Decision: a clear project request starts without asking, with a one-line notice of what and where (stoppable, renamable); a named location in the prompt is used; if unsure whether it is a project, the assistant asks. In away mode no questions are asked at all; the assistant decides and picks the location.

- Decision: the core serves the browser UI over HTTPS with a self-created certificate (browsers only allow their crypto on secure pages); the pairing PIN also confirms the certificate fingerprint; the browser creates a non-exportable key after the PIN and proves possession automatically on each visit; paired devices are listed with revoke; the UI says a browser is less safe than a native app. This resolves the browser-UI auth question.

- Decision: the hosted relay and subscription are a separate track (own repository and timeline, no dates) started after the first release; the first release works fully without it; settings show its features as 'coming later'. Covers accounts, billing, relay hosting, hosted search, memory sync, support.

- Decision: for now the only way to extend the app with running code is MCP servers (plus data-bundle plugins).

- Correction (user): skills and MCP servers are separate things from plugins. A plugin bundle holds workers, departments and themes. Skills and MCP servers are added on their own; how is still open.

- Research (corrects my earlier simplifications): an Agent Skill is a folder with SKILL.md plus optional scripts/, references/, assets/ (code can ride along); an MCP server is a program offering tools, resources and prompts, either local (started as a process) or remote (HTTPS), not just a pointer. Skills/MCP stay separate from plugins; the project-folder trust rule applies to them (user: yes). Open: how skills and MCP servers are added, which MCP features are supported.

- Decision: skills use the open Agent Skills format unchanged, previewed with scripts listed before acceptance. MCP servers are added by link (remote) or by command (local, showing the exact command); tools first, resources/prompts/elicitation later. A project folder's skills and MCP servers load only after trust; the trust pop-up has a button to disable the folder's skills and MCP servers and use the user's own.

## Plan review (user answered the review page; 9 agreed, 5 disagreed)
- Agreed as written: product, failures, away mode, memory, endpoints, permissions and privacy, devices, channels, stack and timeline (note 'Also' was left unfinished).
- Correction: worker parallelism is adjustable (sequential, queues, or N at a time).
- Addition: workers down the line can request permission from higher-ups (sudo, out of folder scope); accepted or rejected up the chain; top is assistant, then user or away mode. This softens the folder lock from 'refused' to 'refused unless a higher level accepts'.
- Correction: no need to see the OmniRoute animation; the flow diagram is a normal flow diagram.
- Addition: custom themes (liquid glass, Frutiger Aero), community themes, and following Omarchy theming, in the app and clients. Format open.
- Question from the user: what 'MCP client only' means (answered in chat; no change).

- Decision (user: 'everything follows the theme, so why break it?'): on Omarchy the whole app follows the Omarchy theme, like Omawrite, Omacalc, Omarchy plugins, the top bar and the browser; no part opts out. Checked on the dev machine: themes in ~/.config/omarchy/themes/<name>/ with colors.toml; current theme under ~/.local/state/omarchy/current/theme; apps join via a .tpl template in ~/.config/omarchy/themed/ and/or a hook in hooks/theme-set.d; shell.toml has background-alpha for translucency; Omarchy blocks code-running files in installed themes. Open: how our theme format describes effects beyond colour.
- Decision: on Omarchy, blur comes from Hyprland's blur settings (hyprctl getoption decoration:blur:*). LastHarness has its own custom theme format (colours, opacity, blur, etc.), not an extension of colors.toml; on Omarchy its values are filled from the active Omarchy theme and Hyprland. Open: the full field list of the format.
- Decision: development is internal and in stealth. Git stays local only, with no GitHub or other remote, until the first public release.
- Decision: a bare backup repository lives on the user's external 2 TB Seagate drive (exFAT, /run/media/omar/Seagate/LastHarness.git), added as the remote `backup`, pushed manually. No network remote until release.
- Rule (user): never commit as Claude. Commits carry the user's identity only, with no Co-Authored-By or Generated-with lines naming an agent.
- Clarification (user): 'stealth' only means the code is not on GitHub or any public host yet. It does not forbid using model providers or other tools during development.
- Rule (user): Claude Code only builds this repository. fable-foreman delegates to its Claude agents only, never to Codex, Grok or other agents.

## Milestone 0 results
- Spikes 1 to 5 ran. Tool calls work natively on all 7 llama-swap models; thinking arrives in reasoning_content; tool_choice none and forced tool choice are ignored. Terminal front end: 3.3 MB at start, 5.7 MB after 30 turns. Embeddings find 9 to 10 of 10 paraphrases in English and Arabic, keyword alone 4 of 10. SQLite handles 100,000 pages. Append-only prompts cut extraction cost 4x to 20x. Not done: Ollama or LM Studio run, cloud run.
- Decision (user: 'do what you recommend'): the local store is SQLite and the embedding model is EmbeddingGemma 2. Open: a native Arabic reader should check the test set; hybrid fusion needs tuning.
- Decision (user): Milestone 1 starts now.
- Decision (user: 'do what you recommend'): the local API uses plain HTTP on localhost with server-sent events for streaming. Checkpoints are committed as the user and pushed to backup without asking each time.
- Decision (user, leaving): the agents keep building without waiting; the user will design the GUI and branding together with the planning session on return. Until then no visual design or branding is invented. Questions that need the user go to .grill/parked.md.
- Correction (user): "no code extensions" meant "not yet", not "never". Spec §9.2 and the roadmap now say code extensions are later, not planned-never; Pi's package catalog (pi.dev/packages) is the model to study then. The first release is unchanged.
- Decision (user: 'Yes!'): the terminal app starts with either `lastharness` or `lh`, both the same program. Known clashes for `lh`: Debian Live's live-helper wrapper and the `lh` directory-lister crate; no clash on the dev machine. No `lastharness` found on GitHub, npm or crates.io. Open: package-database check before release.
- Decision (user): real-model test runs for non-coding work (privacy judge, learning, extraction, chat) use Qwen3.6-35B-A3B-NanoPlus-fast; Tiel-Coder and Ornith are for coding only. Recorded in AGENTS.md.
- Decision (user: 'Yes!'): the app gets a built-in base prompt placed before the learned and about-you blocks. It states the role, working folder and mode, permission rules in plain words, how to use memory and tools, and how to handle a refusal. Learning never rewrites it. Departments' roles build on it in Milestone 4.
- Decision (user): the base prompt must stay short, because a large system prompt raises time to first token on local models. Checked Pi (local docs and binary): its base is one sentence of role, then a few guideline lines contributed by each enabled tool (so disabled tools add nothing), context files, and skills as name, description and path only, with full skill text loaded on demand. Tool schemas go in the request's tools list. LastHarness follows the same shape and sets a token budget with a test.
- Decision (user): the system prompt stays small for the whole development, not just at the start. Standing rule in AGENTS.md and spec 4.3: new features justify any always-on text, prefer on-demand loading, stay within a tested budget, and report the measured size each milestone.
- Decision (user): the base prompt is dynamic. Its first line names the model (with parameter count when the server reports it) and the provider it runs through, for example "You are Qwen3.6-35B-A3B running through llama-swap." It is fixed when the session starts and rewritten only when the model or provider changes (one cold prefix re-read). Each department worker gets its own line for its own model. Where the server does not report a name or size, the line uses what is known and omits the rest; it never guesses.
- Decision (user asked what happens when details are missing): the model line is chosen from complete templates, never assembled with gaps: model and provider known; model only; provider only; neither ("You are an AI assistant."). Parameter count is added only when reported. The model name falls back to the id with path and file extension stripped. A test covers every combination and checks that no line has an empty slot or a dangling "running through".

## 2026-10-11: GUI toolkit test approved

The user approved the toolkit test week (Slint, egui, iced, GPUI; Arabic text, screen reader, memory, size, Wayland, tray, mobile reuse). The build session runs it as a throwaway spike under `spikes/gui-toolkits/`. The look and branding are designed by the user with the planning session in the meantime. Toolkit choice is not decided; it waits for the results.

## 2026-10-11: GUI feel and themes

The user agreed the app should feel calm and quiet by default: lots of space, one accent colour, chat as the main thing, departments and memory in a hideable side panel. The user also wants full theme support: colours, backgrounds, button animations, and whole looks such as Liquid Glass, Glassmorphism, Frutiger Aero and Matte. The toolkit test now also checks blur, translucency, gradients, animation, runtime theme files and background media. Whether themes are loaded from data files is open until the test results are in.

## 2026-10-11: GUI toolkit chosen: GPUI

The toolkit test ran on Slint, egui, iced, GPUI and Qt Quick (results in `spikes/gui-toolkits/RESULT.md`). The user chose GPUI. Idle memory: GPUI 60 MB, Slint 73 MB, egui 118 MB, Qt 130 MB. The user rejected web-view shells (Tauri, Qt WebEngine) and closed or paid UI engines. Themes therefore cannot use real CSS in the native app; the theme file format stays open for the design talks.

Open risks for GPUI: screen reader (AccessKit is in Zed's repository but not in the crates.io release), right-to-left layout (the app must mirror it itself), no Android or iOS support, the pinned gpui-component needs a local patch for an old dependency, and the theming tests (blur, gradients, live theme file, tray icon) were not done. The base prompt now says LastHarness is built by LastSoftware.

## 2026-10-11: Theme file format

The user chose TOML for theme files: plain text, editable and shareable. A theme sets colours, background, corner roundness, blur, shadows and animation. Built-in themes ship with the app and users can write their own. The browser page may also accept an extra CSS file later (not decided).

## 2026-10-11: Built-in themes

The user chose four built-in looks, each with a light and a dark version: Calm (plain default), Matte (flat, solid colours), Glass (translucent with blur, like Liquid Glass and Glassmorphism) and Frutiger Aero (glossy, bright, soft gradients). Glass depends on GPUI blur, which is not tested yet.

## 2026-10-11: Main window layout

The user chose three parts: a slim chat list on the left, the chat in the centre, and a right panel for departments, memory and settings. Every part must be expandable and hideable. A department view can be widened, or opened as its own page from the sidebar, and the chat keeps running while it is open. Things the agent opens (web views, artifacts and similar) show in the side panel and can be expanded or popped out as a separate window.

Open: the user rejected web-view shells for the app itself, so how agent-opened web content and artifacts are drawn inside a GPUI app is not decided.

## 2026-10-11: Web content in the app, Servo test

Agent output is shown in our own native viewers first (markdown, code, images, tables, diagrams). The user asked for a non-Chromium web view and chose to test Servo (Rust engine, on crates.io as v0.1.0 since April 2026, not 1.0, breaking changes expected monthly, half-yearly LTS line). Gecko has no supported desktop embedding and Ladybird is pre-alpha, so neither is an option now. Whether Servo ships, and as an optional add-on, waits for the test.

## 2026-10-11: Resume a chat with a different model

A chat can be resumed with a different model than it started with. A warning appears first. For a local model it says the first reply will take a long time because the whole chat must be read again, and shows how much context the chat holds. For a cloud model it says this will spend credits, because the whole chat is sent again. Open: whether the cloud warning shows an estimated cost.

## 2026-10-11: Save the KV cache to continue chats faster

The user wants the model's KV cache (prefill) saved per chat so a chat can be loaded and continued faster. Checked: llama-server has `--slot-save-path` and slot save and restore (the local build is b11541, and the llama-swap config already mentions slots). A saved cache only works with the same model file and settings, so it does not help when a chat moves to another model (that case still uses the warning). Cloud providers keep their own prompt cache, so nothing is saved by us there. A saved cache contains the chat's content in model form, so it counts as private data: same protection as the chat store, deleted when the chat is deleted or forgotten, and skipped for incognito chats. Open: automatic or manual, and a size cap.

## 2026-10-12: Chat tree like Pi's /tree

The user asked for Pi's `/tree` feature. Checked in Pi's docs (sessions.md, keybindings.md): a session is a tree of entries. `/tree` moves within the same session file; selecting an earlier user message puts its text back in the editor so a new branch starts when it is sent; selecting an assistant entry continues after it. The model receives only the active branch. When you leave a branch Pi can summarise it and attach the summary to the branch you enter. Nodes can carry labels, the tree can be folded and filtered (for example hide tool results). `/fork` makes a new session from an earlier message and `/clone` copies the active branch. LastHarness gets the same: branches inside one chat, a tree view in the desktop app and the terminal, optional branch summary, labels, fork and clone. Open: whether memory learning reads only the active branch (recommended) and whether the branch summary is on by default (recommended: ask each time).

## 2026-10-12: Show HTML pages in the side panel

The user wants HTML pages drawn inside the side panel, and asked for lightweight ways besides Servo (146 MB binary, 133 to 156 MB per page). Candidates to test, none chosen yet: Blitz (Rust, uses Stylo, Taffy, Vello, Parley; HTML and CSS without JavaScript; crates were alpha in May 2026 and the README is inconsistent on maturity), litehtml (C++ engine with Rust bindings, no JavaScript as far as found, good for simple pages) and Servo as the helper process (full pages with JavaScript). The existing card with "Open in your browser" stays as the fallback. Pages made by the agent are usually static, so no JavaScript may be acceptable. Servo stays optional.

## 2026-10-12: Desktop redesign approved

The user reviewed the redesign mock and said yes to all four parts: a slim left icon rail with a lighter title bar (chat title, branch, context meter, model; theme and text direction move to Settings and the palette); assistant answers as plain text in a column about 640 px wide with bubbles only for the user; thinking and tool calls collapsed into one line such as "Worked for 14 s, 3 steps", with permission requests as a Yes / No / Always card; and a Ctrl+K command palette. The right side becomes one dock with Opened, Departments, Memory and Tree tabs. Motion: ease-out, 140 ms presses, 220 ms panels, off under reduced motion. Branding (typeface, logo, accent colour) is still the user's to design with the planning session. The build session was told to work in slices and not to touch the mouse or keyboard.
