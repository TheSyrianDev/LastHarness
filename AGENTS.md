# LastHarness (agent instructions)

A department-led, local-first, endpoint-agnostic AI harness with a memory that lasts. Rust core with a local API, thin front ends. Nothing is coded yet: the project is at the start of Milestone 0 (spikes).

## Read first, in this order

1. `docs/digest.md`: the whole plan on one page.
2. `docs/roadmap.md`: what gets built and when. Milestone 0 is next.
3. `docs/spec/lastharness.md`: what the product is. The spec wins over anything you remember.
4. `.grill/lastharness-reboot.md`: every decision, in order, with the user's corrections. Read it before proposing anything that might already be decided.

`docs/archive/` and `research/` are reference only. The old design in the archive is not the plan.

## Rules for this project

- **Stealth.** Git is local, branch `main`. The only remote is `backup`, a bare repository on the user's external Seagate drive (`/run/media/omar/Seagate/LastHarness.git`), pushed manually. Never add a network remote, push over the network, or create anything on GitHub until the user says release has started.
- **Never commit as Claude.** Commits are authored by the user, with the user's git name and email. Do not set Claude or any agent as author or committer, and do not add `Co-Authored-By`, "Generated with" or any other line that names Claude or another agent, in commit messages or anywhere else in the history.
- **Decisions are the user's.** Do not record a proposal as a decision. If something is undecided, ask one question at a time, with a recommended answer worded so that "yes" accepts it. Keep the spec, roadmap, digest and decision log in step when a decision lands.
- **Spikes are throwaway.** Milestone 0 experiments live under `spikes/`, answer a question, and are not product code. Report each result in plain language and wait for the user before moving on.
- **Check facts, do not assume.** When unsure how something works (MCP, Agent Skills, Omarchy, a model's behaviour), read the source or test it, and say what was verified.
- **Tone.** Calm, plain language, short. No em-dash asides. The user is learning along the way and is allowed to be unsure.

## Environment

Linux (Omarchy, Arch), Dell G15 with 24 GB RAM and an RTX 3050 with 4 GB. Rust comes from the Arch package (no rustup yet). llama-swap runs on port 9292 and models are in `~/AI`. Pi is installed locally for reference.
