# Muse Code — Feature Dossier (THIN-SOURCE)
Registry: #84 | Category: 01 | Verified: closed beta, no repo; musecodes.io is an UNOFFICIAL fan site (footer: "This is an unofficial fan site, not affiliated with Meta Platforms, Inc."); fetched 2026-08-24

Skip-confirmation dossier. Recorded disposition: "Closed beta, no documented mechanism" (09-feature-extraction.md #84), with the data-quality flag that musecodes.io is a fan site (99-corrections.md). This dossier CONFIRMS both facts and extracts what little mechanism detail the fan site carries at marketing level — extract less, never pad. Nothing here is first-party Meta documentation.

## Feature inventory
- **Product identity (as claimed by fan site)**: "Meta's first AI coding agent, in your terminal," beta for macOS/Linux as of August 2026, powered by Muse Spark 1.2; install via `curl -fsSL https://dev.meta.ai/install.sh | bash` with Meta Developer account sign-in [REPORTED-unverified:https://musecodes.io — unofficial fan site]
- **Multi-agent fan-out in isolated git worktrees**: "plans the work, fans it out to persistent background agents in isolated git worktrees"; workers + background reviewers; worktrees merge only after main-agent review [REPORTED-unverified:https://musecodes.io]
- **Event log / replay**: every tool call and decision written to a full event log ("12:04:03 agent-2 edit src/billing/api.ts +42 −17"); watch live, pause or redirect an agent mid-task, replay any run step by step afterward [REPORTED-unverified:https://musecodes.io]
- **Model-harness co-training claims**: Muse Spark 1.2 "co-trained with the Muse Code harness itself — rejection-sampled agent trajectories, goal conditioning, and context compaction"; 1M-token context; "1,000+ tool calls, sustained runs up to 24 hours" [REPORTED-unverified:https://musecodes.io]
- **Training-data pricing split**: Contributor tier ($0.10/1M input) permits session use for product improvement; Standard tier promises data never used for training [REPORTED-unverified:https://musecodes.io]
- Provenance guard: footer disclaims Meta affiliation; no official Meta documentation was reachable from this site beyond the dev.meta.ai install link it displays

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Whole product | ❌ Conflict-skip (confirmed) | §16.2 SDK lock + §16.1 boundary | Closed-source vendor harness tied to a proprietary model; not adoptable as a substrate under the one-worker-category rule regardless of capability |
| Persistent background agents across a session | ❌ Conflict-skip | §16.5 ephemeral dispatch | Standing workers violate ephemeral dispatch; LastHarness's long-work pattern is Automation-dept scheduled re-checks against stored goal state (§14.6) |
| Worktree fan-out with review-gated merges | 🧩 Mechanism only (already spec'd) | §16.6 input 2 + verify-before-promote §11.3 | Third independent confirmation this wave (#79 Superset, #49 checkpoints) that per-dispatch worktrees plus merge-after-review are the working pattern — already LastHarness's dispatch contract |
| Full event log with mid-task pause/replay | 🧩 Mechanism only | Immutable decision log §12 / run records §16.6 output 3 | The pause-and-replay UX is the owner-facing view of what LastHarness grounds in the append-only SQLite log; replay-from-log is citable as a decision-history consumer feature, kernel-side |
| Harness co-training (rejection-sampled trajectories, goal conditioning) | 🧩 Mechanism only | Not actionable at V1 | Model-training technique, outside LastHarness's no-training scope; noted only because "goal conditioning" rhymes with §14.6 goal-state re-entry |
| Contributor-tier training on user sessions | ❌ Conflict-skip | §2.1 privacy-first / egress default-deny | The exact bargain LastHarness refuses by default; local inference exists so code never leaves the boundary |

## Notable absences
THIN-SOURCE held: no documented permission/approval model, no memory or context-management detail beyond "context compaction" as a training phrase, no security posture, no self-host story (naturally — closed beta), no audit of who reviews merges. Everything above is fan-site marketing prose; zero first-party mechanism documentation was available. Do not upgrade any bullet to VERIFIED without an official Meta source.
