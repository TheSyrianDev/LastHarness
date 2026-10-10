# Vercel Open Agents — Feature Dossier
Registry: #37 | Category: 01 | Verified: 5,787★, MIT, last push 2026-08-20 (repo-verify.txt 2026-08-24)

Vercel's open-source template/reference app for building and running background coding agents on Vercel: Next.js web UI → durable Workflow SDK agent → isolated Vercel Sandbox VM, with GitHub App integration from prompt to PR. §C conflict recorded: cloud background-agent execution model.

## Feature inventory
- Three-layer reference architecture: web app (auth, sessions, chat, streaming UI) → agent as durable workflow → sandbox as execution environment (filesystem, shell, git, dev servers, preview ports); "meant to be forked and adapted" [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Key stated design decision — "the agent is not the sandbox": the agent runs OUTSIDE the VM and interacts through file-read/edit/search/shell tools; agent execution untied from request lifecycle, sandbox hibernates/resumes independently, model/provider and sandbox evolve separately [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Durable multi-step execution: chat requests start Workflow SDK-backed runs instead of inline execution; each agent turn persists across many workflow steps; reconnectable streams resume active runs; cancellation supported [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Isolated Vercel Sandboxes with snapshot-based base images + resume (`VERCEL_SANDBOX_BASE_SNAPSHOT_ID`), hibernate-after-inactivity, exposed dev-server ports 3000/5173/4321/8000 [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Repo cloning and branch work inside the sandbox [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Optional auto-commit, auto-push, and PR creation after a successful run — explicitly preference-driven, not always-on [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Chat-driven tool suite: file, search, shell, task, skill, and web tools [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Session sharing via read-only links [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Optional voice input via ElevenLabs transcription [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Auth stack: Better Auth with Vercel + GitHub OAuth social providers; GitHub App installation tokens for repo access/pushes/PRs; webhook secret handling [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- Managed cloud infra: Neon Postgres (auto-provisioned via deploy button), optional Upstash Redis/KV as skills-metadata cache with in-memory fallback [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- packages/agent contains subagent and skills implementations alongside tools [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]
- `OPEN_AGENTS_RESOURCE_PROFILE=hobby` deployment profile switching to resource-constrained defaults for chat and sandbox resources [VERIFIED-fetched:https://raw.githubusercontent.com/vercel-labs/open-agents/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Cloud background-agent execution (Workflow SDK + Vercel Sandboxes) | ❌ Violates no-cloud-execution invariant (§16.1 cloud-worker exclusion, 2026-08-11) | — | The recorded conflict: WHERE does execution happen? On Vercel's infrastructure, full stop |
| Agent-outside-sandbox separation ("the agent is not the sandbox") | 🧩 Pure architecture pattern | §16.5 worker/worktree split — validating principle | Same topology LastHarness already locks: kernel control plane distinct from execution environment; third-party validation of the spec's shape |
| Durable step-persisted workflows w/ resumable streams + cancel | 🧩 Pattern only | Decision-log-backed dispatch records §12 + dispatch flow §4.6 | Persistence/resume/cancel semantics are right; implementation needs no workflow engine — immutable SQLite log gives replay and audit for free |
| Snapshot-based sandbox hibernate/resume | ❌ Tied to Vercel Sandbox product (§16.1) | Compliant analog: preserved dirty worktrees | Compute state is discarded between dispatches; only the worktree (with uncommitted changes preserved) survives — nothing VM-shaped needed |
| Repo cloning + branch work in sandbox | ⚙ Capability Development dept owns | Worktree fan-out §16.1/§7.1 | Identical capability executed in the local env on kernel-managed worktrees |
| Preference-gated auto-commit/push/PR | ⚙ Capability Development dept owns | Commit-per-changeset worker contract §16.5/§16.6 | Commit discipline adopted wave-1; push/PR egress allowed only through a github.com-allow-listed gateway route |
| File/search/shell/task/skill/web tool kit | ⚙ Capability worker contract owns | Deny-by-default allow-lists §16.5/§14.5 | Standard worker kit; every tool gated per dispatch, skill tool reads the registry |
| Session sharing via read-only public links | ❌ Exposes owner work to external viewers via cloud URLs (self-hosted boundary §2.1) | Compliant analog: local read-only console views | Sharing happens on the LAN/local console against the decision log, not via hosted URLs |
| Voice input via ElevenLabs | ❌ Cloud transcription of voice violates local-first inference (§16.1) | OS-native speech APIs (#117 Yap adopt precedent) | Delegate to OS-shipped speech recognition — zero weights loaded, zero egress |
| Better Auth social login + GitHub App installation tokens | 🧩 Pattern only | Auth chain §13; tokens in §14.5 vault | App-token pattern for repo access transfers; credentials kernel-held, never agent-held |
| Neon Postgres + Upstash KV managed data planes | ❌ Violates self-hosted boundary (§2.1/§16.1) | SQLite + FTS5 already the stack | Single-owner scale needs none of it; skills-metadata cache falls back to memory even upstream |
| Subagents + skills packages | ⚙ Capability existing structures own | Role tables §14.2 / skills registry §14.5 | Already-specified nouns; no new mechanism |
| Hobby resource profile | 🧩 Pattern only | Hardware-tiered model packs §17.3 | Independent convergence: deployment profiles keyed to available resources = pack tiering by hardware |

## Compliant re-implementation sketch
- Keep the three-layer topology, executed entirely locally: local web owner console → kernel dispatch ledger → ephemeral Claude Agent SDK workers in git worktrees on THE one Linux box.
- Elevate "the agent is not the sandbox" into a stated §16.5 principle: control plane (kernel) never lives inside the execution environment, and no agent process ever runs inside a worktree — the kernel reaches in through bounded handles.
- Replace Workflow SDK durability with decision-log-backed dispatch records (§12): each step appended immutably, resume = replay-from-log, cancel = kill worker + append outcome; no separate orchestration engine.
- Replace snapshot hibernate/resume with dirty-worktree preservation: teardown commits nothing but keeps uncommitted changes on disk for inspection or re-dispatch.
- Auto-commit/auto-push/auto-PR become worker-contract options: commit-per-changeset mandatory inside the run; push/PR only via the gateway route allow-listed to github.com using GitHub App credentials from the §14.5 vault.
- Map the tool kit one-to-one onto deny-by-default allow-lists (file/search/shell/task/skill/web), with `web` egress-gated like everything else; skill resolution hits the §14.5 registry.
- Swap ElevenLabs voice input for OS-native speech APIs (Yap precedent): transcription stays on-device.
- Stay out: Vercel Workflows/Sandboxes, Neon Postgres, Upstash KV, public share links — each named above; SQLite+FTS5 and local read-only console views cover every function at single-owner scale.

## Notable absences
No local/self-hosted execution path documented — sandboxes exist only as Vercel Sandboxes. No memory layer at all (nothing persists knowledge across sessions beyond git state). No mid-run approval gates — preferences gate end-of-run actions only, otherwise cancel is the sole control. No cost/token telemetry mentioned despite being a commercial vendor's reference app. No egress policy for in-sandbox tools (web/shell unrestricted once running). Developer-first fork-and-adapt docs; no non-technical operator path.
