# LastHarness Specification

**Version:** 0.1 draft. Nothing is built.
**Companion documents:** [`../roadmap.md`](../roadmap.md) says when each part is built. `.grill/lastharness-reboot.md` records why each decision was made. The old design is archived in [`../archive/lastharness-v1-spec.md`](../archive/lastharness-v1-spec.md).

Each section is marked **Decided** (the user chose it), **Provisional** (a recommendation the user has not rejected) or **Open**.

---

## 0. Document Map

| § | Topic |
|---|---|
| 1 | Vision and audience |
| 2 | Principles |
| 3 | Architecture at a glance |
| 4 | The core: lifecycle, idle mode, local API |
| 5 | Providers, endpoints and model types |
| 6 | Memory |
| 7 | Learning: rules and skills |
| 8 | Roles, planning and autonomy |
| 9 | Tools and permissions |
| 10 | Privacy, trust levels and spending |
| 11 | Front ends |
| 12 | Devices, pairing and remote access |
| 13 | Settings and onboarding |
| 14 | Platforms, release and licence |
| 15 | Out of scope |
| 16 | Open decisions |

---

## 1. Vision and Audience — Decided

LastHarness is a harness for AI models. It is **department-led**, **local-first** and **endpoint-agnostic**.

- **Department-led.** Work is done by a team of specialist workers organised into departments (§8). You hand it a goal; the departments do the parts.
- **Local-first.** Your models and your data live on your machine by default. Cloud models and services are optional additions.
- **Memory underneath.** It remembers who you are, how you work and what you decided, so you stop re-explaining yourself. It learns your habits (for example "always give links after research", or "install software through the Omarchy commands") and applies them. Each worker runs on this memory.
- **Endpoint-agnostic.** It sits on top of any model server: Lemonade, Ollama, llama.cpp, LM Studio, llama-swap, any OpenAI-compatible or Anthropic-style server, and any cloud provider including OpenRouter. It does not run or install models.
- **Versatile.** Terminal, browser, native desktop app, phone app and chat apps all talk to the same core.

**Audience.** Technical users: people on Linux, people who build good slide decks in an Office suite, people who have used ChatGPT or DeepSeek, and people who run local models in LM Studio. It should also be gentle enough for someone starting with AI.

**Why specialists.** Models do poorly when asked for a product, website, design, branding, marketing, video and images all at once, especially small local ones. Giving each kind of work to a specialist role does better (§8).

---

## 2. Principles — Decided

1. **The user decides.** Anything that changes behaviour or sends data somewhere new is a proposal the user confirms. Relaxing that is a setting.
2. **Everything is a setting.** Each behaviour choice is made at onboarding with a safe default and is changeable later.
3. **Memory is local.** Cloud models see only what the task needs. Sensitive items never leave.
4. **RAM belongs to the models.** The harness stays light, and idle costs almost nothing.
5. **Own harness.** It is not a control plane for other agents.
6. **Open and checkable.** The code is open source so the privacy promise can be verified.
7. **No invented cryptography.** Use established protocols and formats.

---

## 3. Architecture at a Glance — Decided (stack) / Provisional (store)

```
 terminal   browser   desktop GUI   phone app   chat channels
     \         |           |            |            /
      +--------+-----------+------------+-----------+
                           |  local API (authenticated)
                 +---------+---------+
                 |       core        |   Rust
                 |  memory · rules · |
                 |  roles · planner  |
                 |  tools · privacy  |
                 +---------+---------+
                           |  provider layer (compat table)
        +--------+---------+----------+----------------+
     llama-swap  Ollama  LM Studio  OpenAI/Anthropic-style  OpenRouter, cloud
```

- **Core in Rust**, exposing a local API. All front ends are thin clients.
- **Local store.** SQLite (decided after the Milestone 0 spike: 100,000 pages gave a 482 MB file, a 25 ms keyword query and a 70 ms plain vector scan; no corruption after kill -9).
- **OS-specific code** (paths, shell, key storage, process control) sits behind one thin layer so the core ports to macOS and Windows without a rewrite.
- Platforms: development is Linux-only and private; the first public release ships on Linux, macOS and Windows (§14).

---

## 4. The Core — Decided

### 4.1 Lifecycle

- The core is **always on** and starts at login.
- Closing a front end collapses it to the **system tray** (or runs quietly if there is no tray). The core keeps running.
- After a **user-set idle time** the core enters **idle mode**.

### 4.2 Idle mode

Idle mode uses almost no resources: no timers, no polling, event-driven wake-up (the idea behind Spotifast's idle behaviour: stop keeping a resource busy when nothing needs it). It drops caches and the in-memory memory index.

Only these stay open in idle:

- **The authentication listener.** A paired, verified device authenticates automatically and a success wakes the core (§12). Unknown devices are rejected at the handshake, before any data is parsed.
- **Enabled messaging channels.** Each holds a quiet outbound connection. A message from an allow-listed sender wakes the core. Messages from anyone else are ignored (§11.5).

Idle CPU wake-ups and RAM are measured, not assumed.

### 4.3 Local API

Every front end drives the core through one authenticated local API. The browser UI is served by the core on the machine's address. Nothing in the API is front-end-specific. Transport: plain HTTP on localhost, with server-sent events (SSE) for streaming.

**Command names.** The terminal app starts with `lastharness` or with the short form `lh`; both run the same program. `lh` clashes with a Debian Live wrapper and the `lh` directory-lister crate, so the short name is a convenience a user can skip. Checked by web search only; the name check before release must cover package databases.

### 4.4 Context management — Decided

When a conversation nears the model's context limit, the core **auto-compacts** it. The context length at which compaction starts is a setting the user can adjust. Summaries are appended, so the conversation prefix stays identical and the server's prefix cache is kept (§6.3).

### 4.5a Owners and scopes — Decided

The first release has one owner per install. From day one, every memory item, rule, session, setting and log entry carries an **owner** and a **scope** (personal, project or team). **Multi-user mode** comes after launch, modelled on Microsoft Teams: Owner, Member and Guest roles; personal and project memory kept apart; each request built only from the asker's memory and projects. The host owner can see everything unless per-person encryption is added later. Members are told this plainly.

### 4.5 Projects and modes — Decided

- A **project is a folder.** Opening the harness in a folder attaches that project's memory and decisions. Global memory applies everywhere.
- **Folderless mode:** plain chat, or a **general assistant** that starts in the user's home folder and does tasks for them. Assistant mode uses the same permissions as everywhere (§9.1).

---

## 5. Providers, Endpoints and Model Types

### 5.1 Providers — Decided

A provider is a base URL, a key, and a model list. Kinds: OpenAI-compatible, Anthropic-style, Lemonade, Ollama, llama.cpp (not app-managed), LM Studio, llama-swap, OpenRouter and other cloud services, and any custom endpoint.

- Keys are stored in the OS secret store, never in plain text in config.
- **Capability probing** records what each model supports: tools, thinking, context size.
- Agents may select models, operate them (load, unload, change settings) and propose new providers. **Registering a provider or key needs the owner's approval by default.**

### 5.2 Endpoint differences — Decided

The approach is the one Pi, Codex and OpenCode use:

- Use the server's **native tool calls**. There is no prompted tool-call fallback.
- A **compatibility table** per provider and per model, auto-detected from the URL and provider, overridable per model. It covers reasoning format, developer role, token-limit field and similar flags.
- Accept reasoning in any of the common field names.
- Repair partial streaming JSON tolerantly.

The server's chat template does most of the work, so a poorly configured server gives a poorer experience.

### 5.3 Model types — Decided

Roles use **model types**, like Opus, Sonnet and Haiku:

- Three stock types: **frontier** (the best model available), **mid-tier**, and **fastest**.
- The user can create custom types (for example *Coder* → Ornith 1.5 35B A3B, *Researcher* → Apodex 1.1 mini) and can add, edit or delete any type, including the stock ones.
- A role is assigned to a type or directly to a specific model. A type can point at any model the user has available, cloud or local.
- Deleting a type that roles use shows a warning and pop-ups to reassign each affected role before the deletion goes through.
- At onboarding the harness may suggest assignments from the models it finds. The user confirms; nothing is ranked automatically.

### 5.4 When a model is unavailable — Decided

A setting with two modes:

1. **Stop** with a clear message and a button to pick another model now (default).
2. **Fall back** along an ordered per-type list. It never falls back to a model the sensitive-data rule forbids, and it tells the user.

### 5.5 Media generation — Decided

Image and video generation are available from the first release through providers (OpenRouter and other cloud services, plus local endpoints). It is switched on or off at onboarding and in settings.

---

## 6. Memory

### 6.1 Architecture — Decided

Memory is a remix of several existing designs (Claude's memory summary and per-project spaces, TencentDB Agent Memory's layers and drill-down, Karpathy's LLM wiki, Mem0's never-overwrite rule, Claude-Mem's progressive search). Its parts, from most stable to least:

1. **About you.** A short block that is always included in a request. It is small, so it is cheap on slow local prefill. It is **assembled, never regenerated**: each accepted page gets a one-line gist written once, in the end-of-session review turn that already runs, and the block is those lines picked by pins and relevance up to a size cap the user sets. When a page changes, only its own line is rewritten, so the cost scales with what changed.
2. **Pages.** Preferences (how you work), facts about you, and projects and decisions, as linked Markdown files with an index file per layer (§6.5).
3. **Conversations.** The raw record, kept in the database, never edited, searchable, and exportable to Markdown on request.
4. **Behaviour.** Rules, the style profile and skills (§7), kept apart from what is known about you.

Any summary or page can be traced down to the conversation that supports it. Everything has an owner and a scope (§4.5a). Projects have their own memory, separate from other projects and from personal memory.

### 6.2 Capture — Decided

- Raw conversations are always stored and searchable, but are never promoted to facts without approval.
- After a session the harness **proposes** facts and decisions. The user accepts, edits or drops each one. "Remember this" works at any time. A setting can enable auto-accept later.
- **Forgetting:** ask the assistant ("forget X"); it shows what was removed. Or use an **incognito session**: a temporary session that stores and learns nothing.

### 6.3 Cost of extraction — Decided (approach), measured in Milestone 0

Local models have slow prefill, so the end-of-session review must not re-read the whole conversation cold:

- Build prompts **append-only** so the conversation prefix stays identical and the server's prefix cache is reused.
- Ask the extraction questions as a final turn.
- Use server slot save/restore (llama.cpp) where capability probing finds it.
- Otherwise extract in small pieces during the session or when idle.

### 6.4 Changed facts — Decided

When a fact changes, the old fact is **archived**, not overwritten or deleted. Archived facts are left out of normal recall, stay searchable and can be restored. The user can still delete anything on request.

### 6.5 Memory as files — Decided

The user can export and import memory, edit it as files, or tell the agent to edit it.

- Each page is **one Markdown file** with a small header (id, type, privacy class, source, dates) and the text. Related pages link to each other, and each layer has an **index file** that doubles as the cheap list for search.
- If the user edits a file, the change is applied when detected, the old version is archived, and the edit goes in the activity log. The user can also create or delete files.
- **Gists and maintenance changes are proposals.** They never overwrite text the user wrote or edited.
- Export writes the folders to a path the user chooses, and import reads them back.

### 6.6 Maintenance pass — Decided

A maintenance pass looks for contradictions, duplicates, stale facts and orphan pages (the idea is Karpathy's "lint"). It runs **on demand** by default (a "tidy memory" command), and a schedule for idle time is a setting, because it costs tokens and is slow on local models. It only **proposes** changes, which the user confirms like any other proposal.

### 6.7 Search and storage — Decided (shape), Open (model and store)

Search is **hybrid**. Keyword search is always on. **Semantic search** (so that "sadness" also finds "sad" and "cry") uses a small multilingual embedding model that ships with the core and runs on the CPU. It loads on demand and unloads in idle mode. The model is EmbeddingGemma 2 (Apache 2.0, 295 MB), chosen after the Milestone 0 English and Arabic test; the Arabic test set still needs a native reader. The user can point it at their own embedding endpoint instead, and embeddings never go through a cloud endpoint unless the user allows it. Results from both are merged and shown progressively: a cheap list first, expanding only what the task needs. Memory sent per request is bounded. Every change is recorded in an append-only history. The store is SQLite.

---

## 7. Learning — Decided

Modelled on Prime Agent's Continual Harness: evidence-backed updates to editable supplemental state. **The base prompt is never rewritten.** Snapshots and rollback are always available.

### 7.1 Rules

- A rule is *how the user wants things done* ("always give links after research").
- Rules are proposed from **repetition across sessions** (threshold N, default 3, user-settable) and from **explicit signals** (corrections, "always" and "never", thumbs).
- The model confirms the pattern is real and writes the rule text and evidence. The user then accepts, edits or rejects it.
- Stated rules also go through confirmation. Silent learning is an opt-in setting.
- The most specific applicable rule wins.

### 7.2 Style profile

- Tone and manner (terse or chatty, how much to explain, whether to ask before acting) live in one short, editable **style profile** that applies to every role. Rules stay for task habits.
- The profile is learned and changed through the same proposals as rules: evidence, confirmation, snapshots and rollback.

### 7.3 Skills

- A skill is a reusable procedure for a type of task. In the open Agent Skills format (agentskills.io) it is a **folder**: a required `SKILL.md` (YAML header with `name` and `description`, plus optional `license`, `compatibility` and an experimental `allowed-tools`, then Markdown instructions) and optional `scripts/` (executable code), `references/` (documents read on demand) and `assets/` (templates, images, data). So a skill is not always just a Markdown file; it can carry code.
- Skills load in three steps to save context: name and description at startup (about 100 tokens each), the full `SKILL.md` when the skill is used, and the other files only when needed.
- A skill's scripts run through the normal tools, under the permission rules (§9.1).
- After a complex task the harness proposes a skill with its evidence. Improvements to a skill during use are also proposals.
- Unlike Hermes, which grades itself without a gate, every skill is confirmed by the user, versioned and revertible.

---

## 8. Roles, Planning and Autonomy — Decided

### 8.1 Roles

- A role is a Markdown file: a header (name, model type or model, tools, sampling, memory slice) and the instructions as the body. The files are the source of truth; the app edits them through a form or chat.
- Changes to a role proposed by the harness use the same confirm-and-rollback path as rules.
- A starter set ships: researcher, writer, coder, designer, planner.

### 8.1a Departments

- A department is a named group of workers (roles) with a **lead**. The lead takes a goal, breaks it into steps and assigns each step to a member. Independent steps can run at the same time (§8.2).
- Departments can nest: a department may contain sub-departments.
- A department is a Markdown file like a role (name, lead, members, instructions), so it can be shared in a plugin bundle (§9.2).
- The plan the user approves and the autonomy level (§8.3) sit on top of whatever the lead plans.

### 8.1b Seeing a department work

- Graphical front ends show a live **flow diagram**: the goal enters, the lead splits it, steps flow out to worker nodes tagged with their model, active nodes glow, and results flow back to the lead. Inspiration: OmniRoute's flowing provider animation.
- Terminal and screen readers get a plain per-worker list (model, current step, status). Any worker can be opened, paused or stopped.
- Approval prompts name the worker that asked.
- The look is a normal flow diagram. Its details are settled when the GUIs are built; there is no need to wait for any reference animation.

### 8.1c When a worker fails

- A worker that fails or gets stuck reports to **whoever called it**, and the caller works out how to resolve it.
- A department lead has several options: do the step itself, spawn a regular subagent for it, or hand it to another worker.
- The caller can also **change the model** for the step and try again.
- The same applies up the chain: a department that cannot resolve a failure reports to whoever called that department.
- If a failure reaches the top unresolved, the top-level assistant tells the user what failed and what was tried, and waits for instructions.
- Each step has a cap on attempts (including retries and spawned subagents) so a stuck department cannot loop or run up cost. Default 3, a setting.

### 8.1d How a department communicates

- The lead gives each worker a short brief: its step, the files or results it needs from earlier steps, and the memory slice its role allows.
- A worker returns a result plus a short summary. The lead reads summaries, not full transcripts, to keep its context small, and can open a full transcript when needed.
- Workers do not message each other. Everything goes through the lead.

### 8.1e Quality checks

- The lead checks each worker's summary against the step it assigned.
- If a result misses the requirements only through small mistakes, the lead **fixes them itself** or asks a worker to fix them. That can be the same worker as a fresh instance with an empty context. Nothing is redone from scratch.
- If the result is wrong in a larger way, it counts as a failure and follows §8.1c (retry, change the model, do it itself, spawn a subagent, hand off).
- Whether the lead uses a stronger model for checking is up to its role settings. It is not forced.

### 8.1f Projects and the project leader

- The user talks to the regular assistant. If a message is a request to make a project, the assistant recognises it without being told to create one.
- The assistant then spawns a **project leader**: a clone of itself with the same tools and the same permissions, with two limits. It cannot spawn another project leader, and it cannot do anything outside the project's folder.
- The project leader acts like a CEO. It calls the departments and runs the making of the project. Departments, failures and quality checks work as in §8.1 to §8.1e, with the project leader as the top caller.

- The folder limit is a **lock**: file system and shell access outside the project folder is refused by default. Web, memory and settings are not confined.
- **Permission requests go up the chain.** Any worker, down the line, can ask its caller for permission to do something it may not do on its own, such as running a command with `sudo` or touching something outside the folder's scope. Each level accepts, rejects, or passes the request up. At the top, the project leader asks the assistant, and the assistant asks the user, or, when the user has set an **away mode** (human unavailable, away from the computer), decides on its own (§8.1g). The answer comes back down: accepted or rejected.

- **Starting a project:** when a request clearly asks for a project, the assistant does not ask first. It says in one line what it is creating and where, and the user can stop or rename it. If the request names a location, the project goes there. If it is unclear whether the user wants a project, the assistant asks.
- Creating the folder follows the normal permission rules.
- In away mode the assistant asks no questions, because nobody is there to answer. It decides on its own (§8.1g) and picks the location itself when none is given.

### 8.1f-2 Departments outside a project

- The user can ask the assistant for a department directly. A small job runs in the current folder with no project leader, and the assistant is the caller. The usual permission rules apply.
- The project leader and its folder lock are only for requests to make a project.

### 8.1g Away mode limits

- In away mode the assistant follows the permission rules the user already set, and the user's allow rules still count. What those rules would ask the user, the assistant decides using what it knows about the user.
- It never approves anything in a sensitive category and never sends sensitive data to a cloud endpoint or a channel.
- `sudo` is not a hard ban here. It follows the `sudo` setting (§9.1), and when it is available the assistant uses it only when needed and with caution.
- Every decision made alone goes to the activity log and is listed in a summary when the user returns.
- If something cannot be decided within these limits, that part of the project is parked and the rest carries on.

### 8.2 Execution

Workers may run **concurrently**. Each worker uses its own model or shares one with others. The scheduler runs steps in parallel when the endpoint or hardware allows it (a cloud endpoint, several loaded models, or a server with parallel slots) and queues them when it does not. Whether concurrency pays off is a property of the setup, not a rule of the product. On the dev rig, parallel streams on one local model did not pay (1.05x for two, 0.88x for three) and a model swap costs about 14 seconds, so there the planner groups steps by model. The parallelism is adjustable: workers can run one at a time, in queues, or several at the same time (3 at a time, for example). The limit is a setting, with a default detected from the endpoint.

### 8.3 Planning and autonomy

Big goals work as: the harness proposes a breakdown, the user approves or edits it, then it runs with milestone pauses. Autonomy is a setting: **plan-first**, **milestone check-ins**, or **fire-and-forget**.

---

## 9. Tools and Permissions — Decided

### 9.1 Permissions

- **Default: tiers.** Reads are automatic. Writes inside the project ask once per session. Anything outside the project, deleting, installing and network access always ask. **`sudo` is off for agents by default**; enabling it is offered at onboarding. When enabled it asks like any other command and can be covered by allow rules; the bypass mode skips the prompt but does not turn `sudo` on if it is off in settings. This is the same permission system in every mode, including the general assistant that starts in the home folder, in the manner of Claude Code's default mode, with the bypass mode below as the opt-out.
- Per-command **allow rules**, as in Claude Code, can be promoted (for example allow `omarchy-pkg-add`). Promotion is itself a confirmed rule.
- **Ask for everything** is a selectable setting.
- **Bypass:** `--dangerously-skip-permissions`. What it covers is a setting with three levels: everything off; tool permissions only; tool permissions plus hard locks (the sensitive-data rule, silent key registration). The default is the third. Using the bypass shows a clear warning that the user accepts the risk and the project is not responsible for the outcome.
- No sandbox is planned.

### 9.2 Plugins and MCP — Decided (MCP client, data plugins; code extensions later)

**MCP is client-only.** The harness uses MCP servers as tools; it is not an MCP server. The model case is a hosted MCP server such as Higgsfield's, which gives an agent image and video generation through one endpoint. Adding an MCP server needs approval, as with a provider. Its tools go through the permission tiers, and the sensitive-data rule applies to whatever the agent sends it. Agents send prompts and arguments, never a memory dump.

**Plugins** are anything that extends the app. In the first release a plugin is a **bundle of data**: workers (roles), departments (groups of roles, optionally nested) and themes. It is installed from a folder or a link. The app shows what a bundle contains before the user accepts it, and a folder's own plugins, skills and MCP servers load only after the user trusts that folder (the idea is Pi's project trust).

**Skills and MCP servers are separate things from plugins.** They are added on their own, not as bundle contents.

- **Skills** use the open Agent Skills format as it is, so skills written for Claude Code and similar tools work unchanged. A skill's contents are previewed before the user accepts it, with any scripts listed.
- **MCP servers** are added as a remote server by link, or a local one by command. For a local one, the approval shows the exact command that will run. Tools are supported first; resources, prompts and elicitation are added when needed.
- **Project folders** load their own skills and MCP servers only after the user trusts the folder. The trust pop-up has a button to **disable that folder's skills and MCP servers and use the user's own** instead.

**What an MCP server is.** An MCP server is a program that gives an AI app context and actions over the Model Context Protocol (JSON-RPC). It can offer three kinds of thing: **tools** (functions the model can call), **resources** (data such as files or database records) and **prompts** (reusable templates). A server can run in two ways: **locally**, where the app starts it as a process on the user's machine (stdio), or **remotely**, where it runs elsewhere and the app connects over HTTPS (Streamable HTTP, with OAuth or API keys). A local server is a program running on the user's machine, so adding one is closer to installing software than to adding a link. A server may also ask the user for input (elicitation). Sampling, where a server asks the app's model for completions, is deprecated in the 2026-07-28 revision. Which of these LastHarness supports is still to be decided.

**Code extensions** in the manner of Pi packages (TypeScript modules that hook into the harness, installed from npm or git, running with the user's permissions) are **not in the first release**, by your choice of "not yet". They may come later, and Pi's package catalog (extensions, skills, prompt templates and themes installed from npm) is the model to study then. For now the only way to extend the app with running code is an MCP server, under the permission rules.

Editor integration stays out of scope.

### 9.3 Web search and fetch — Decided

Web fetch and web search are built-in tools. The search provider is chosen at setup, as an essential choice: none; a paid search API with the user's key; the subscription's search; an existing SearXNG instance; an app-managed SearXNG instance (setup shows its RAM cost); or a free provider such as TinyFish. Browser automation is not in the first release.

---

## 10. Privacy, Trust Levels and Spending

### 10.1 Sensitivity — Decided

- Default sensitive categories: credentials, health, finance, identity documents, and anything marked private. Detection uses simple pattern rules plus model judgment when an item is saved. Uncertain items count as sensitive and appear in the review pass.
- The user can reclassify any item.
- A **"show me what was sent"** view lists exactly what left the machine.

### 10.2 Trust levels — Decided

Each provider and each messaging channel has a trust level (local only, own cloud account, any cloud). An item goes only to a destination whose level is high enough. Messaging channels sit at a low level and carry no security promise.

### 10.3 Spending — Decided

Three modes, all selectable at onboarding and changeable later: **no limits**, **hard caps**, and **estimate-and-approve** with thresholds. The default is estimate-and-approve.

### 10.4 Activity log — Decided

One append-only local log records every tool call, every request that left the machine, and every rule, skill, role or setting change, with undo where it makes sense. It backs the "show me what was sent" view.

---

## 11. Front Ends

**Themes (all front ends).** The app and its clients can select custom themes, such as liquid glass or Frutiger Aero, and can download themes made by the community (themes are one of the contents of a plugin bundle, §9.2). On Omarchy, **everything follows the Omarchy theme**, the way Omawrite, Omacalc, the Omarchy plugins, the top bar and the browser do. There is no part of the app that opts out.

What Omarchy provides (checked on the dev machine, to be re-checked against the Omarchy source when this is built):
- A theme is a folder in `~/.config/omarchy/themes/<name>/` with a `colors.toml`, backgrounds and an icon theme. The active theme is under `~/.local/state/omarchy/current/theme`.
- An app joins the theme system with a **template file** in `~/.config/omarchy/themed/` (for example `app.json.tpl`, using placeholders such as `{{ background }}`, `{{ foreground }}`, `{{ accent }}` and `{{ mix background foreground 12% }}`), which Omarchy renders when the theme changes. A **hook** in `~/.config/omarchy/hooks/theme-set.d/` can also run on every theme change.
- `shell.toml` sets translucency (`background-alpha`) for the bar, menu and notifications, which is how glass looks are done.
- Omarchy refuses code-running files in themes installed from outside (terminal configs, Lua, editor extensions) and keeps only colour. This matches our rule that plugins are data only.

So LastHarness ships a template and a hook, takes its colours, fonts and translucency from the active theme, and switches live when the theme changes. **Blur on Omarchy comes from Hyprland's own blur settings** (readable with `hyprctl getoption decoration:blur:size`, `blur:passes` and `blur:enabled`; checked on the dev machine), so the app's blur matches the rest of the desktop.

**Theme format (Decided).** LastHarness has its **own theme format**, not an extension of Omarchy's `colors.toml`. It describes colours, opacity, blur and similar settings (the full list of fields is still to be written). On Omarchy the app fills those values from the active Omarchy theme and Hyprland. On macOS and Windows it uses themes from its own theme folder and follows the system light or dark setting. Community themes are plugin bundles carrying these files, with no code.

### 11.1 Terminal — Decided

First front end, and the first thing worth showing to anyone.

### 11.2 Browser UI — Decided

Served by the core at the machine's local network address. Responsive, so it also works in a phone browser over the local network or the user's VPN. Behind the same pairing as every other device (§12). Accessible by default.

### 11.3 Native desktop GUI — Decided (approach)

No Electron. Test Slint, egui, iced and GPUI against RAM use, right-to-left and Arabic text shaping (translations are planned), and screen-reader support (Orca on Linux, VoiceOver on macOS, Narrator or NVDA on Windows). Fall back to a Tauri-style wrapper only if none passes.

### 11.4 Mobile app — Decided

A native thin client for Android and iOS, in the first release. Pairing, key handling and the protocol live in one **Rust client library** shared by both phones through **UniFFI** (the approach Bitwarden, Signal and Firefox use). The UIs are native and thin: SwiftUI on iOS, Jetpack Compose on Android. A week-1 spike in the mobile milestone verifies Keychain and Keystore key storage, push notifications, and Arabic text before committing. The device key lives in the phone's secure storage. Chat, plan approval, rule and memory review, and notifications. 

### 11.5 Messaging channels — Decided

- First release: **Telegram, Matrix, Signal and Slack**. **WhatsApp is out for good**: no channel, no skill, no support (the unofficial route risks bans; the official one needs a public webhook).
- Every channel is **opt-in** and off by default. Slack shows a privacy warning at connection. Each Slack user creates their own app, which avoids the Marketplace rate limits.
- Channels carry no security promise and never carry sensitive memory (§10.2).
- Only allow-listed senders are accepted.
- No editor integration. MCP and plugins are supported (§9.2).

---

## 12. Devices, Pairing and Remote Access

### 12.1 Pairing — Decided

Modelled on Sunshine and Moonlight, which pair with a PIN and then use certificates:

1. The new device (phone or browser) asks to pair and shows a PIN.
2. The user enters the PIN on the machine, in the terminal, tray or UI. This proves the user controls the already-authenticated machine.
3. The devices exchange keys. From then on the device authenticates by its key, automatically, with no password each time.
4. Devices are listed and can be unpaired at any time.

A native app keeps its key in the phone's secure storage. A browser keeps a key in the browser, which is weaker; the settings say so plainly.

**Browser details (Decided).** Browsers only allow their built-in cryptography on secure pages, so the core serves the browser UI over HTTPS with a certificate it creates itself. The first visit shows a one-time browser warning, and the pairing PIN also confirms the certificate fingerprint, as Sunshine does. After the PIN, the browser creates a key that cannot be exported and keeps it. Each later visit proves possession of that key automatically, with no password. The settings list every paired device with a revoke button.

### 12.2 Connection — Decided

Direct first (same network, or a VPN the user already runs), then a **relay** as a fallback. The relay passes only encrypted data and cannot read it; the relay software sees device identities and addresses. Syncthing works the same way.

### 12.3 Subscription — Decided

A paid LastHarness subscription covers three optional services: the **hosted relay**, **hosted web search** and **memory sync across devices** through a login. The relay code is open source and self-hostable. LastHarness itself is free and works fully without an account. Synced memory is encrypted with a key derived from the account password, so the servers hold only ciphertext, and a recovery code is offered at signup. Losing both loses only the synced copy.

The subscription is a **separate track with its own repository and timeline**, started after the first release. The first release works fully without it. Until it exists, pairing works on the local network or the user's own VPN, and the settings show the subscription features as "coming later". The track covers accounts, billing, relay hosting, hosted search, memory sync and support.

### 12.4 Security notes — Provisional

- The authentication listener is the one open door in idle. It must reject unknown keys at the handshake before parsing anything (a flaw of this kind was fixed in KDE Connect in 2025, where device identity was exchanged without authentication).
- The relay does not hold messaging credentials; channel connections are always made from the user's own machine.

---

## 13. Settings and Onboarding

### 13.1 Settings — Decided

Every behaviour choice is a setting: learning mode, autonomy level, media on or off, spending mode, permission mode and bypass scope, model-unavailable behaviour, idle time, channels, and device pairing. A single settings surface shows all of them.

### 13.2 Onboarding — Open

Onboarding sets these at first run with safe defaults. How it presents them is decided last. The candidates are one-choice **profiles** (Careful, Balanced, Hands-off) with an advanced view, a **step-by-step** flow, or **just-in-time** questions. The leading idea is profiles for the first run plus just-in-time questions for anything not covered.

### 13.3 Updates and crash reports — Decided

- Updates are on by default, cryptographically signed, and can be turned off in settings.
- Anonymous crash and bug reports are opt-in, asked at onboarding. The user can preview exactly what would be sent, and a setting allows automatic sending. There are no usage analytics.

---

## 14. Platforms, Release and Licence

- **Development:** Linux only, and private until the first public release: the code is not on GitHub or any public host yet. Git is local, with one backup remote that is a plain folder on the user's external drive.
- **First public release:** Linux, macOS and Windows together, with the mobile app on Android and iOS.
- **Open source** from the first release. The harness, plugin format and apps are **Apache-2.0**. The relay server is **AGPL-3.0**, in a separate repository, so a competitor cannot run a closed hosted fork; self-hosting stays free.
- The roadmap, including the effort for each of these, is in [`../roadmap.md`](../roadmap.md).

---

## 15. Out of Scope — Decided

- Controlling other agents (Claude Code, Codex, Pi, Hermes). That is Zeron and MonoCode territory.
- An installer, hardware detection, model packs and benchmark sweeps. The harness does not manage inference.
- Editor integration.
- Electron.
- WhatsApp (never).
- A sandbox for tools.
- Home-made cryptography.

---

## 16. Open Decisions

1. ~~The embedding model and the store~~ decided: EmbeddingGemma 2 and SQLite.
2. Onboarding presentation (decide last).
3. The name is LastHarness ("the last harness you'll need"). Before release, check the domain, package names and trademark (Harness is also a DevOps company).
4. Whether the spec should gain detailed sections (data formats, API schema) as the design firms up.

