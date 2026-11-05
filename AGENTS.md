# LastHarness (agent instructions)

A department-led, local-first, endpoint-agnostic AI harness with a memory that lasts. Rust core `lhd` with a local HTTP API and SSE, thin front ends: terminal (`lh`) and the GPUI desktop app (`crates/lh-gui`).

## Read first, in this order

1. `docs/digest.md`: the whole plan on one page.
2. `docs/roadmap.md`: what gets built and when. Check the current milestone in the Next step section below.
3. `docs/spec/lastharness.md`: what the product is. The spec wins over anything you remember.
4. `.grill/lastharness-reboot.md`: every decision, in order, with the user's corrections. Read it before proposing anything that might already be decided.

`docs/archive/` and `research/` are reference only. The old design in the archive is not the plan.

## Rules for this project

- **Release has started (2026-10-17).** The user approved publishing the code to GitHub. Pushing to GitHub is allowed once the user says the repository is ready; do not create the GitHub repository or push without that go-ahead for each first push. The `backup` remote on the user's Seagate drive (`/run/media/omar/Seagate/LastHarness.git`) stays. Before anything goes public, check it holds no secrets, tokens, private paths or personal data.
- **Never commit as Claude.** Commits are authored by the user, with the user's git name and email. Do not set Claude or any agent as author or committer, and do not add `Co-Authored-By`, "Generated with" or any other line that names Claude or another agent, in commit messages or anywhere else in the history.
- **Commit as you go.** Commit automatically after each finished, working change (it builds and its tests pass), with a short message that says what changed and why. Stage only the files you changed for that task, never `git add -A` over the user's other uncommitted work. Do not push unless the user asks. The never-commit-as-Claude rule above still applies.
- **Decisions are the user's.** Do not record a proposal as a decision. If something is undecided, ask one question at a time, with a recommended answer worded so that "yes" accepts it. Keep the spec, roadmap, digest and decision log in step when a decision lands.
- **Spikes are throwaway.** Milestone 0 experiments live under `spikes/`, answer a question, and are not product code. Report each result in plain language and wait for the user before moving on.
- **Check facts, do not assume.** When unsure how something works (MCP, Agent Skills, Omarchy, a model's behaviour), read the source or test it, and say what was verified.
- **Keep the system prompt small.** A large system prompt slows time to first token on local models. Every feature that wants to add text to the system prompt must justify it, prefer on-demand loading (tools, skills, memory recall) over always-on text, and keep the measured size within the budget test. Report the size in each milestone report. Never trim silently and never grow silently.
- **Never take over the user's mouse or keyboard.** Do not run scripts that move the real pointer, click, or type into the user's desktop (the GUI window driver in `crates/lh-gui/tests/gui_drive.py` is one). Run it only when the user says so for that run. Test the app with unit and backend tests, or ask the user to try it.
- **Delegate fixes.** The user wants code fixes and edits done by subagents (Claude agents), not by the main session. The main session plans, checks results and reports.
- **Share the machine.** Write your name to `~/AI/.machine-busy` before builds or model loads, and clear it after. Other sessions use the same lock.
- **Parked means parked.** The web UI is archived at tag `web-ui-archive`; do not extend it unless asked. Held for last: /refine and rewards, Strata, the native phone app.
- **Keep shell commands safe.** Do not put `pkill -f` or `pgrep|kill` in the same command that restarts the thing you killed (it kills its own shell). Start servers in a separate command with `setsid`.
- **Tone.** Calm, plain language, short. No em-dash asides. The user is learning along the way and is allowed to be unsure.

## Environment

Linux (Omarchy, Arch), Dell G15 with 24 GB RAM and an RTX 3050 with 4 GB. Rust comes from the Arch package (no rustup yet). llama-swap runs on port 9292 and models are in `~/AI`. Pi is installed locally for reference.

## Next step

Milestone 0 (spikes) is done. Milestones 1 to 8 are built (see `docs/milestone-N-report.md`; 4 is roles and departments, 5 is onboarding and settings, 6 is the browser UI (now parked), 7 is messaging channels, 8 is the desktop app in `crates/lh-gui`, drawn with GPUI). Early releases (v0.1.0-alpha.x) are on GitHub for Linux, Windows and macOS; the public repo is `~/Projects/public/LastHarness`. On 2026-10-30 the user kept GPUI after trying Tauri 2 (generative answers drawn natively, apps in Servo) and chose the "Quiet desk" redesign; see the decision log. Milestone 9 (mobile) and 10 (macOS and Windows) are next; wait for the planning session. Check `git log` and `.grill/parked.md` for the current state. Decided so far: SQLite store, EmbeddingGemma 2, local API over HTTP on localhost with SSE, the terminal app starts as `lastharness` or `lh`. While the user is away, ask nothing and park questions in `.grill/parked.md`. GUI visuals and branding: propose creative designs freely, and the user decides. Not yet done: the cloud API run (ask first), and Ollama and LM Studio probing.

## Test model

For real-model runs of any work, use `LFM3-1.7B-balanced` on llama-swap for now. It is small, so give it generous token limits and do not expect strong reasoning.
