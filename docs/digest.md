# LastHarness in one page

Details: `spec/lastharness.md` (what it is), `roadmap.md` (when), `../.grill/lastharness-reboot.md` (every decision, in order).

## What it is

An AI harness, like Claude Code, Codex or Pi, but **team-first** (departments and their workers do the work), **local-first** and **endpoint-agnostic**, with memory as a supporting feature. It is for technical users: Linux users, Office power users, people on ChatGPT, DeepSeek or LM Studio. The app is named Last AI ("The last AI app you'll need"), made by Last (the company; mission "the last software you'll need"). Its engine is Last Engine. Inside the app: Studio (image and video, via ComfyUI), Search (SearXNG) and Crew (the user's workers). The CLI is `lastai`, with `lh` as an alias for one release. Model downloads are part of the Models page, not a separate product. It is free, open source (PolyForm Noncommercial 1.0.0) and works with no account.

## How it works

- **Departments.** You talk to your assistant. It calls departments: named groups of workers (roles) with a lead. The lead splits a goal into steps and hands them out. Workers can run one at a time, in queues or several at once (adjustable), each on its own model or a shared one. Departments can nest.
- **Projects.** If you ask for a project, the assistant spawns a **project leader**, a clone of itself that cannot spawn another leader and is locked to one folder by default. Workers down the line can ask their caller for permission (for example `sudo` or something outside the folder), and each level accepts, rejects or passes it up. It acts like a CEO and calls the departments.
- **Failures.** A stuck worker reports to whoever called it. The caller can retry, change the model, do it itself, spawn a subagent or hand it off. Small mistakes are fixed in place. Unresolved failures go up to you. Attempts per step are capped (default 3).
- **Away mode.** When you are away the assistant asks no questions. It decides within your permission rules, never approves sensitive things, logs every decision and gives you a summary when you return.
- **Memory.** Plain Markdown files you can read, edit and export, plus a short "about you" block, raw conversations in a database, and separate rules, style and skills. Changed facts are archived, never overwritten. Search combines keywords with a small built-in multilingual embedder (EmbeddingGemma 2), stored in SQLite. Incognito sessions store nothing.
- **Learning.** Rules are proposed after a pattern repeats (default 3 times) and the model confirms it. Skills follow the Agent Skills format. Everything learned is confirmed by you and revertible.
- **Endpoints.** Any OpenAI-compatible or Anthropic-style endpoint, llama-swap, Ollama, LM Studio, OpenRouter. Model types (frontier, mid-tier, fastest, plus your own) let roles ask for a kind of model, not a name.
- **Permissions and privacy.** Reads are automatic. Writes in the project ask once per session. Outside the project, deleting, installing and network always ask. You can ask for everything, set allow rules, or bypass with a warning. `sudo` is off for agents unless you turn it on. Sensitive data never goes to cloud endpoints or messaging channels. One activity log records everything.
- **Extending.** MCP servers (client only), skills, and data-bundle plugins (workers, departments, themes). No code extensions yet. A project folder's own skills and MCP servers load only after you trust it, and the trust pop-up lets you use your own instead.
- **Themes.** Custom themes (liquid glass), community themes as plugin bundles, and a format of our own (colours, opacity, blur). On Omarchy the whole app follows the Omarchy theme and Hyprland's blur.
- **Always on.** The core starts at login, lives in the tray and goes near-zero idle. Only the authentication listener and enabled messaging channels stay open.
- **Devices.** Paired like Sunshine/Moonlight: a PIN, then keys. Direct connection first (LAN or your VPN). The browser UI is served over HTTPS by the core. Phone apps are native thin clients on a shared Rust library.
- **Channels.** Telegram, Matrix, Signal and Slack, opt-in, low trust. WhatsApp: never.
- **Subscription (later, optional).** Hosted relay, hosted web search and encrypted memory sync. A separate track after the first release.

## Stack and timeline

Rust core with a local API. Thin front ends: terminal first, then browser, then a native desktop app (GPUI, web pages and small apps in a Servo viewer), then phone apps. Development is Linux-only (through Milestone 9) and private, with macOS and Windows built in Milestone 10; the first public release ships on Linux, macOS, Windows, Android and iOS together.

| Milestone | Weeks |
|---|---|
| 0 Spikes | 1–2 |
| 1 Core and terminal chat | 3 |
| 2 Memory v1 | 3 |
| 3 Learning v1 | 3 |
| 4 Roles, departments and planning | 3 |
| 5 Onboarding and settings | 2 |
| 6 Browser UI | 3 |
| 7 Messaging | 3 |
| 8 Native GUI | 5 |
| 9 Mobile app | 7 |
| 10 macOS and Windows | 5 |
| 11 Release | 2 |

About 40 weeks. This is a lower bound: search, the activity log, auto-compaction, projects, MCP and departments are not fully counted.

## Still open

- **Onboarding presentation:** decided last, on purpose.
- **Name check:** domain, packages and trademark for Last, Last AI, Last Engine and the inside names before release. CLI name set: `lastai`, `lh` alias for one release.
- **Themes:** the format, and how Omarchy theming is followed.
- **Optional:** detailed spec sections (data formats, API schema).

## What happens next

Milestone 0, five throwaway spikes under `spikes/`, each answered with numbers: (1) chat and tool calls against three servers (llama-swap, Ollama or LM Studio, one cloud API); (2) Rust terminal app RAM, idle and in a long session; (3) memory retrieval, keyword only versus keyword plus a small embedding model, in English and Arabic; (4) the local store decision, with SQLite as the default candidate; (5) the cost of end-of-session extraction on a local model. The spikes are done: SQLite and EmbeddingGemma 2 are chosen, and Milestone 1 has started. Git is local, with one backup folder on the Seagate drive.
