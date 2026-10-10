# Buzz — Feature Dossier
Registry: #87 | Category: 06 | Verified: 30,334★, Apache-2.0, last push 2026-08-24 (repo-verify.txt 2026-08-24)

Block's self-hostable team workspace built on a Nostr relay: humans and AI agents share rooms where every message, reaction, workflow step, review approval, and git event is one signed event in one searchable, hash-chained log. Recorded conflict: peer channels violate §4.4 kernel-mediation + no-persistent-inbox.

## Feature inventory
- Self-hostable relay workspace: humans + AI agents in the same rooms; every message/reaction/workflow step/review approval/git event is a signed event in one log — same identity model and audit trail for people and processes [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- Agents as first-class members: own keypairs, own channel memberships, own audit trail; "scoped by identity, not by permission flags — the same way you'd scope a teammate"; full surface parity (channels, canvases, workflows, huddles) [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- Channels/threads/DMs/canvases/media/frame-anchored-comments/search/audit log working today; desktop app (Tauri + React); mobile clients pending [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- Agent surface: `buzz-cli` (JSON in/out, designed for LLM tool calls) plus ACP harness wiring Goose, Codex, and Claude Code into the relay; agents can watch channels, pull history, run first-pass reviews, orchestrate other agents [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- YAML workflows triggered by message / reaction / schedule / webhook events [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- Git integration: NIP-34 events (patches, repo announcements, status), nostr-signed git (`git-sign-nostr`/`git-credential-nostr`), git hosting backend; "branch as room" — patches, CI results, review, merge decision co-located per feature branch [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- Architecture: Rust relay (Axum WS+REST, NIP-01/NIP-42 auth) over Postgres (events + FTS search), Redis (pub/sub, presence, typing), S3/MinIO (Blossom media), hash-chain audit log crate; single source of truth is the relay [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- Tenancy model: default single-relay-single-community; hosted multi-community keeps tenant-scoped rows/caches/search/workflow state per host-derived community [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- One-click Railway deploy for hosted relays [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]
- Unified search across conversation, patch, workflow run, and approval in one place (Postgres FTS over the single event log) [VERIFIED-fetched:https://raw.githubusercontent.com/block/buzz/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Relay channels/DMs as the inter-agent coordination plane (agents watching channels, prompting via messages) | ❌ Violates kernel mediation (§4.4) + no-persistent-inbox (§16.5) | — | WHERE does execution happen? In whoever reacts to a channel first; channel-watching agents are standing listeners with peer-to-peer authority — exactly what §4.4 routes through the kernel instead |
| One signed event log w/ hash-chain audit for humans AND agents | 🧩 Pure architecture pattern | §12 decision log | Strongest alignment in this wave: append-only, tamper-evident, identity-attributed records of every agent action is what LastHarness's decision log already mandates |
| Per-agent cryptographic signing identity | ⚙ Capability Security dept owns | §12 attribution + §13 identity model | Per-agent keys strengthen decision-log attribution; but authorization stays role-config-derived (§16.6), never membership-derived |
| Identity/membership-based scoping ("own channel memberships") | ❌ Violates deny-by-default tool allow-lists (§16.6) | — | Capability must come from a role's config slice decided by policy, not from which rooms an agent was added to; room membership as permission is ambient authority |
| YAML workflows: schedule triggers | ⚙ Capability Automation dept owns | §14.6 job_runner | Scheduled automation is already LastHarness's sanctioned background-motion mechanism |
| YAML workflows: message/reaction/webhook triggers | ❌ Violates ephemeral dispatch (§16.5) | — | Event-triggered firing requires resident listeners on live channels (or inbound webhook ingress) — standing processes watching for stimuli |
| Git events co-located with discussion/approval per branch | 🧩 Pure architecture pattern | §12 linkage + §6 episodic memory | Dispatch records linking to repo artifacts (commit/PR ids) give every decision its evidence trail without any new transport |
| Unified FTS across conversation/code/workflow/approval | 🧩 Pure architecture pattern | §14.3 Information retrieval surface | Search spanning decision log + department memory is a genuine gap candidate; Buzz proves demand and the single-log shape that makes it tractable |
| Postgres + Redis + MinIO multi-service stack | ❌ Violates stack discipline (§16.1: ONE self-hosted Linux env, SQLite) | — | Four standing infrastructure services exceed the single-SQLite-database-per-installation boundary |
| ACP harness hosting Goose/Codex/Claude Code clients | ❌ Violates SDK lock (§16.2) | — | Foreign harnesses are excluded substrates; LastHarness workers are Claude Agent SDK subagents |
| Desktop/mobile client apps | ⚙ Capability owner console owns | Owner-console presentation | Multi-surface presentation of one event stream; no architectural content beyond what the console already renders |
| Railway one-click cloud deploy | ❌ Violates self-hosted/no-cloud-execution (§16.1/§2.1) | — | Agent workload execution on managed third-party cloud is categorically excluded |

## Compliant re-implementation sketch
- Rebuild the valuable core — *every human and agent action lands as one signed, attributable, searchable record* — inside existing structures rather than a new transport:
  - Give each department role (or manager) a distinct signing identity recorded with every decision-log entry (§12/§13): attribution becomes cryptographic, not merely asserted.
  - Link dispatch records to repo artifacts (commit/PR ids produced by Development work) so "why does this code exist" is answerable from the log — Buzz's branch-as-room provenance without the room.
  - Extend Information-department retrieval (§14.3) to unified FTS across decision log + department memory: conversation, artifact, and approval search in one query.
  - Keep schedule-driven motion exclusively in Automation scheduled re-checks (§14.6); nothing subscribes to anything.
- All coordination stays kernel-mediated (§4.4): an "agent joins the discussion" becomes a kernel-routed dispatch whose output is appended to the record, not a peer posting to a channel.
- Stay out: the Nostr relay transport, channels/DMs as agent surfaces, resident event listeners, the Postgres/Redis/MinIO stack, foreign ACP harnesses, and any hosted/cloud deployment path.

## Notable absences
No egress control or sandboxing story for what hosted agents do outside the workspace; no per-agent budget/quota layer — identity scoping covers *what* agents may touch, not how much. Approval gates for workflows are listed as still being wired up, so the human-authorization path is incomplete at README level. The audit log is hash-chained but externally unverifiable from the README alone (no export/verification tooling mentioned). Vision docs advertise cross-relay reputation and push notifications as pending — correctly not counted here.
