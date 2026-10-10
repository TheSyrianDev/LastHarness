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
