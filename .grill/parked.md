# Parked for Omar

Things that need the user. Agents add to this list instead of asking while the user is away. Nothing here is decided.

- **GUI and branding design.** The user wants to design these together with the planning session once back. Do not invent visual design or branding. Build only functional, unstyled front ends until then.
- **Cloud API run** for spike 1 (sends requests off the machine): needs a go-ahead.
- **Ollama or LM Studio run** for spike 1: neither is installed.
- **Arabic test set** for memory retrieval needs a native reader.
- **Old backup copy** `/run/media/omar/Seagate/LastHarness.git.corrupt-2026-10-10` can be deleted once the user agrees.

## Milestone 1 (added by the build session)

- **Commit and push permission.** The planning session relayed that checkpoints may be committed as the user and pushed to backup without asking. I could not confirm that from my own session, so I have not committed or pushed anything from the build session. The planning session did the commits. Please confirm the rule once, directly.
- **Tray icon.** The roadmap says the core "collapses to the tray when there is one". A tray icon is a visual thing, so I built no tray. The core keeps running without a front end; the icon waits for the GUI design.
- **Start at login.** Not installed. A systemd user unit is the plan (`lhd.service`); installing it changes the user's machine, so it waits for a yes.
- **Spend: unknown prices.** A paid model with no price entry asks in estimate mode and is refused in caps mode. Local and private-network endpoints are free. Day and month limits use UTC boundaries, not local time. All three are my defaults; change if wrong.
- **Spend: no usage report.** If a server sends no token counts, the ledger row has zero tokens and, for priced models, zero cost. It does not guess.
- **Shell commands.** A command the app cannot classify asks every time (not once per session). Bypass has one level only (no prompts); the three bypass levels in the spec need the "hard locks" to exist first, which come with memory privacy in Milestone 2.
- **Compaction.** Runs between turns only, not in the middle of a long tool loop. When the server reports no context size, it starts at 24,000 tokens.
- **Web search.** Only SearXNG is built (checked with a fake server, not a real instance). The paid search API, the subscription search and TinyFish need their API details and keys, so they wait.
- **Web fetch.** Local and private network addresses are blocked for agents on purpose. Say if you want an allow list.
- **Capability probing.** Works for llama-swap and llama.cpp (tested live). Ollama and LM Studio probing is not written; neither is installed here to check.
- **Authentication listener.** The bearer-token listener is what exists. Pairing by PIN and device keys belong to Milestones 6 and 9, so "paired devices wake an idle core" is not built yet.
- **Image and video generation.** See below once done.
- **Video generation.** Not built. There is no common video endpoint across providers, and I could not check OpenRouter's video API from here. Images work through the OpenAI-style `/images/generations` endpoint (tested with a fake server only, not against a real image model).
- **Image prices.** Add `image = 0.04` (dollars per image) to a `[[price]]` entry in the spend settings. A paid image model with no price asks or is refused like any unpriced model.
- **Per-model compatibility overrides.** The compatibility table is detected per provider. A per-model override the user can edit is not built yet.
- **Exit test, cloud half.** Milestone 1 says "a real task finished on a local model and on a cloud model, same session format". The local half passed (see docs/milestone-1-report.md). The cloud half sends requests off the machine and waits for a go-ahead and a key.

## Milestone 2 (added by the build session)

- **Over-cautious privacy judging.** On the real run (Tiel-Coder-35B as the local judge), "Lives in Berlin" and "Prefers short plain answers" were both judged sensitive, because the rule is that an unsure model counts as sensitive. That is safe but noisy: those notes would never go to a cloud model. The review screen lets you flip each one with `p` before accepting, and `lh memory normal <id>` flips it later. Decide whether a place of residence or a style preference should count as sensitive by default.
- **Two privacy classes only.** Notes are Normal or Sensitive. Sensitive goes only to a model on this machine. The trust level "own cloud account" currently behaves like "any cloud". The spec has three trust levels but no third class of note, so I did not invent one.
- **"Forget" deletes for good.** The `forget` tool and `lh memory delete` remove the page, its older versions and its text in the history (the fact that something was deleted stays). Archive is the gentler option and is what a changed fact uses. Say if "forget" should archive instead.
- **Incognito.** No memory is read or written, no conversation is stored, and review is refused. The activity log still records tool names and permission answers, without any text, as a security record. Say if incognito should leave no log at all.
- **Embedding model is not bundled.** Meaning-based search works when `lhd` is given a llama.cpp server and the EmbeddingGemma 2 file (`--embed-server`, `--embed-model`), or an embedding endpoint. Without them, search falls back to keywords only. Shipping the model and server with the app belongs to packaging in Milestone 11.
- **Hybrid weights.** Keyword weight 0.35, meaning weight 1.0. On the small test set (8 paraphrase questions per language, real EmbeddingGemma 2) keywords alone found 3/8 English and 5/8 Arabic in the top 5, and the hybrid found 8/8 in both. Tuned on a tiny set; revisit with real use. A native Arabic reader still needs to check the Arabic test texts.
- **Review editing in the terminal.** The review screen can accept, drop, postpone or flip sensitivity. Editing a suggestion's text happens after accepting, by editing the Markdown file or with the API.
- **Folderless sessions and projects.** Pages of the projects layer attach to the project folder the session was opened in. Folderless sessions have no project.
- **Maintenance on idle.** `--maintain-when-idle` runs the deterministic checks only (duplicates, broken links, stale pages). The contradiction check needs a local model and is on demand.

- **Question for the user: "clippy".** The user told the coding session they had given "clippy as a reference model to embedding models". Nobody knows what this refers to (clippy is only the Rust linter the agents run). Ask the user what they meant.

## Milestone 3 (added by the build session)

- **Near-duplicate suggestions.** Two suggestions that say the same thing in different words can both appear, because they are compared by shared words, not by meaning. In the real run three sessions left two suggestions waiting. Matching by meaning (the embedding model) would merge them; it is not wired in yet.
- **Style note or rule.** The model decides whether "no bullet points" is a style note or a rule. On both Tiel-Coder and Qwen it chose a style note. Both kinds work and can be rolled back; nothing needs deciding unless you want formatting wishes to always be rules.
- **Qwen versus Tiel.** On Qwen3.6-35B (the model AGENTS.md now names) the learning suggestions and the memory review parsed fine, with a 3000 to 4096 token limit. The one difference: for "Lives in Berlin" Qwen's privacy judge said normal where Tiel's said sensitive. Both parsed cleanly. No switching back was needed.
- **Rules and the style profile live in the database, not as Markdown files.** They can be listed, edited and rolled back with `lh learn ...` and the API, and every version is kept. A Markdown mirror that you could edit by hand (like memory pages) is not built.
- **Whole-state snapshots.** Rollback is per item (each rule, the style profile, each skill has its own versions). Restoring everything learned to one earlier moment is not built.
- **Skills cannot be switched off.** They can be rolled back or deleted. A skill only reaches a model if it is valid; broken ones are listed with the reason.
- **When learned things take effect.** Rules, the style profile and the skill list are put into a session's first message, so a change applies from the next session. This keeps the start of every prompt identical for the server's cache.
- **Incognito.** Rules, style and skills still apply (they are how you want things done), but nothing is noted, suggested or learned.
- **Roles.** A rule can name a role, but roles arrive in Milestone 4, so none is set yet.
- **Skill suggestions only from a local model.** The record of a long task includes tool arguments, so skills are suggested (and improved) only when the model runs on this machine, and only after a task of at least 6 tool calls (`--skill-min-calls`).
- **Other skill folders.** Skills in `~/.claude/skills` or inside a project folder are not loaded. The format is compatible (the parser reads the real Omarchy skills), but loading project skills needs the folder-trust step that comes with Milestone 4.
- **Habit threshold.** 3 sessions by default (`--learn-threshold`); the model confirms that a repeated wish is real before anything is suggested.
- **Embedding server and the GPU.** Found and fixed: the embedding server (a CUDA build) crashed when the chat model filled the 4 GB GPU. It now runs with the GPU hidden. Worth knowing if you ever see it fail on another machine.
- **AGENTS.md reading order** still says "Milestone 0 is next" in the list under "Read first". The "Next step" section is current.

- **Seagate drive was not cleanly unmounted** (kernel: "exFAT-fs (sda1): Volume was not properly unmounted. Some data may be corrupt. Please run fsck."). This damaged the backup repository twice. Both times the local repository was intact and the backup was rebuilt from it. Two damaged copies are left on the drive (`LastHarness.git.corrupt-*`). Suggested fix, run by the user: unmount the drive, run `sudo fsck.exfat /dev/sda1`, then delete the damaged copies. Always unmount before unplugging. exFAT has no journal, so a second copy elsewhere would also be safer.
