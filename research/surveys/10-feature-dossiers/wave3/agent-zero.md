# Agent Zero — Feature Dossier
Registry: #136 | Category: 06 | Verified: agent0ai/agent-zero — 18,960★, license NOASSERTION, pushed 2026-08-25 (API, 2026-08-25)

## Feature inventory
- Autonomous agentic framework that "runs on its own computer": a Linux machine (Docker container) with browser, terminal, and its own filesystem [VERIFIED-fetched:https://github.com/agent0ai/agent-zero README + https://www.agent-zero.ai/]
- Web UI for watching the agent work: clicking, typing, running code in the container [VERIFIED-fetched:https://www.agent-zero.ai/]
- Creates and uses its own tools as code during execution; learns and self-corrects across runs [VERIFIED-fetched:https://www.agent-zero.ai/]
- 100+ community plugins/extensions [VERIFIED-fetched:instagram.com/reel/DcVbuv3Dn-m caption + agent-zero.ai]
- Real-file document editing (edits actual files, not chat-bound text) [VERIFIED-fetched:reel caption]
- Multi-agent cooperation: agents can instantiate subordinate agents [REPORTED-unverified — docs pages not fully fetched this pass]
- Docker-isolated execution per framework design; launcher desktop app for install/update/backup [VERIFIED-fetched:https://www.agent-zero.ai/]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Whole-framework adoption (Docker-hosted "own computer") | ❌ Conflict-skip | — | §16.1: third-party container runtime is explicitly ruled out on every platform; LastHarness's ONE boundary is its own WSL2/containerization/nspawn environment |
| Agent-created tools as executable code | 🧩 Mechanism only | §14.5 skills intake w/ harness validation (#108 pattern) | The capability is right; unvalidated self-authored tools entering execution is exactly what the intake lifecycle gates |
| Real-file document editing | 🧩 Mechanism only | Already specced §16.6 worktree discipline | Confirms files-not-chat as the artifact model at product scale |
| Subordinate-agent instantiation | 🧩 Mechanism only | §16.5 scoped children | Legal only when bounded to parent task/budget/lifetime — Agent Zero's version lacks those bounds |
| Persistent memory/self-correction across runs | ⚙ Feature adoption (bounded form) | §10.2 verifier-gated refinement + department memory | Learning loop fits iff proposals stay sandbox-gated (Hermes ruling family) |
| Web UI observation of live agent work | 🧩 Mechanism only | §16.10 owner console render | Strengthens R7: watch-the-work surface over kernel records |

## Notable absences
No policy layer: tool use, network access, and destructive commands are not gated by an allow-list or approval contract — everything the agent decides to do inside its container happens without staged confirmation. No audit trail comparable to an append-only decision log. License NOASSERTION (custom terms — verify before any reuse). Container-per-agent isolation is the architecture's whole security story, which transfers nothing to LastHarness's single-boundary model.
