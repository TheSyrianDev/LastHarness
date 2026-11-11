# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

One design language across three kinds of screen, all thin front ends on the same Rust core: the native desktop app (GPUI, `crates/lh-gui`; Linux now, macOS and Windows in Milestone 10), the browser UI served by the core (`crates/lh-core/web/`), and phone apps (Milestone 9; whether they are native SwiftUI and Compose or a shared HTML front end is still open). The terminal app (`lh`) is a front end too, but outside this design work.

## Users

Primary: people who want a private AI assistant on their own machine. That includes technical users (Linux users, Office power users, people coming from ChatGPT, DeepSeek or LM Studio) and, confirmed 2026-10-30, non-technical people who have never run a local model. The app must make sense to the second group without slowing down the first.

Their job: talk to an assistant all day, hand bigger work to departments, and keep a memory of what matters, without an account and without their data leaving the machine unless they choose.

## Product Purpose

Last is an AI harness, like Claude Code, Codex or Pi, that is team-first (departments and their workers do the work), local-first and endpoint-agnostic, with memory as a supporting feature ("The last AI app you'll need"). Success: a person uses it as their everyday assistant, trusts what it does on their behalf, and never has to open a terminal to set it up or run it.

## Positioning

- **Departments:** the assistant calls named groups of workers with a lead, which split a goal into steps; projects get a project leader locked to one folder.
- **Local-first and private:** works with local models (llama-swap, Ollama, LM Studio) as well as cloud endpoints; sensitive data never goes to cloud endpoints or messaging channels.
- **Memory that lasts:** plain Markdown files the user can read, edit and export; changed facts are archived, never overwritten; anything learned is confirmed by the user and revertible.

## Operating Context

- Always on: the core starts at login, sits in the tray and goes near-zero idle.
- Long sessions with streamed answers, tool steps, permission questions, and department runs that keep going while the user does other things (away mode logs every decision and summarises on return).
- Devices pair like Sunshine/Moonlight (PIN, then keys); the browser UI is reached over HTTPS from the same machine, the local network or the user's VPN.
- Messaging channels (Telegram, Matrix, Signal, Slack) are opt-in and low trust.
- Free, open source (PolyForm Noncommercial 1.0.0), no account.

## Capabilities and Constraints

- Chat with streaming, stop, model per chat, stored chats with resume, a chat tree (branches), command palette, side panel (opened content, departments, memory, branches), settings that never need the terminal.
- Generative answers: the `show` tool can put a table, a chart, or a small sandboxed HTML app into the conversation; an app may offer one follow-up the user sends with a click.
- Model picker grouped by model, with context tier, template and mode inside each (like the `/pick` Pi extension).
- Permission tiers: reads automatic; writes, deletes, installs, network and `sudo` ask according to the rules.
- System prompt budget: features must not add always-on prompt text without justification (cap 300 tokens).
- Measured desktop memory: GPUI app about 136 MB idle.
- Undecided: the phone app approach, onboarding presentation (decided last on purpose), the theme format details, the CLI name (lastai or lh), and the final name check.

## Brand Commitments

- Name: Last, made by LastSoft (decided 2026-11-11). Tagline: "The last AI app you'll need". Inside the app: Studio (image and video, via ComfyUI), Search (SearXNG) and Crew (the user's workers). The CLI name is open. Voice: calm, plain language, short; the user may be unsure and is allowed to be.
- No visuals or branding are invented by agents: the user designs the look; proposals are shown as options to choose from.
- Must keep (confirmed 2026-10-30):
  - Arabic and right-to-left: mirrored layout, Arabic fonts (Noto Naskh Arabic default), English and Arabic mixed in one line.
  - Following the Omarchy theme: on Omarchy the app takes its colours and Hyprland's blur from the system theme.
  - User-made themes: theme files (colours, opacity, blur, roundness, fonts per theme) stay editable and loadable as community plugin bundles.

## Evidence on Hand

- Built-in themes: `crates/lh-gui/themes/` (Calm, Matte, Glass, Grove, Cobalt, each light and dark).
- Milestone reports with measured numbers: `docs/milestone-*-report.md`.
- No users, testimonials, benchmarks against other products, or press exist yet; none may be invented.

## Product Principles

1. Private by default: nothing leaves the machine unless the user chose it, and the app says so plainly when it would.
2. The user stays in charge: everything learned or decided on their behalf is visible, confirmed and revertible.
3. Calm power: a tool someone keeps open all day; depth is there when wanted and out of the way when not.
4. Plain words over jargon, so a first-time user is never stuck (model tiers, tokens and templates need explaining where they appear).
5. One product everywhere: desktop, browser and phone share the same concepts and words.

## Accessibility & Inclusion

- Right-to-left languages, starting with Arabic, are first-class.
- The spec plans screen-reader support (Orca, VoiceOver, Narrator or NVDA) and full keyboard use; the desktop app already exposes an accessibility tree and keyboard shortcuts. Not marked as a binding redesign constraint in the 2026-10-30 interview.
- Reduced motion is respected (no movement when the desktop asks for less).
