# DeerFlow — Feature Dossier
Registry: #113 | Category: 06 | Verified: MIT (badge in README); v2.0 ground-up rewrite; #1 GitHub Trending 2026-02-28 claimed in README; ~80k stars reported by registry, not independently verified

## Feature inventory
- Open-source "super agent harness" orchestrating sub-agents, memory, and sandboxes on LangGraph/LangChain; ships filesystem, memory, skills, sandbox-aware execution, and planning/spawning of sub-agents out of the box [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- Progressive skill loading: Markdown SKILL.md modules loaded only when the task needs them to keep the context lean; skill directory acts as a package boundary so nested SKILL.md files are data, not runtime skills [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- Sandbox execution modes selectable in config: local host execution, isolated Docker containers, or Kubernetes pods via a provisioner service [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- Sub-agent runtime caps such as `subagents.max_total_per_run` exposed in config [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- Long-term memory with Settings > Memory UI, sample-data loader for review, optional OpenViking memory backend plus an explicit model-invoked `forget` MCP tool requiring user confirmation before irreversible delete [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- Session goals, manual context compaction, and context-engineering controls listed as core features [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- Durable run machinery: per-run terminal `run.delivery` receipt persisted before terminal status, lease/heartbeat run ownership across Gateway workers, orphan-run reconciliation, SSE `gap` events instead of silent partial replay [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- SQLite or Postgres unified backend shared by LangGraph checkpointer, store, and app data; checkpoint cache is performance-only and safe to disable [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- `make setup` interactive wizard (provider, search, sandbox/bash/file-write safety choices) + `make doctor` diagnostics + redacted `make support-bundle` for issue filing that never includes `.env` or raw conversations [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- IM channel inbox (Telegram, Slack, Feishu, WeChat, WeCom, DingTalk, Buzz/Nostr) with inbound queue bounds, allowlists (`allowed_users`), and per-user session overrides; scheduled tasks; TUI workbench; embedded Python client; LangSmith/Langfuse/Monocle tracing [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]
- MCP server support: tool-name prefixing to prevent cross-server collisions, per-tool timeouts, OAuth token flows, and adaptation of remote submit/status/cancel tools into durable background tasks with retries, dead-lettering, and cancellation [VERIFIED-fetched:https://raw.githubusercontent.com/bytedance/deer-flow/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Standing Gateway service with SSE streams, IM inboxes, scheduled tasks | ❌ Conflict-skip | — | Standing processes + message inboxes violate ephemeral-worker and kernel-mediated-routing invariants |
| Lazy markdown skill loading with package boundaries | 🧩 Mechanism only | §skill-format | Pure context-hygiene pattern; portable to LastHarness skill loader |
| Docker-container sandbox mode on own host | 🧩 Mechanism only | §worker-runtime/sandbox | Execution stays on self-hosted Linux; matches LastHarness boundary if containers are ephemeral |
| K8s/E2B provisioned sandboxes | ❌ Conflict-skip | — | External/remote execution surfaces |
| `subagents.max_total_per_run` caps | ⚙ Feature adoption | §worker-runtime | Blast-radius cap for ephemeral workers; cheap invariant enforcement |
| Long-term memory store with explicit forget + confirmation | ⚙ Feature adoption | §memory (dept-scoped) | LastHarness wants gated dept-scoped memory; adopt gating+forget UX, not user-global scope |
| Terminal delivery receipt persisted before status; SSE gap events | 🧩 Mechanism only | §decision-log | Same write-ahead honesty as immutable decision log: record what was delivered, never replay silently |
| Redacted support-bundle generator (`make support-bundle`) | ⚙ Feature adoption | §support/diagnostics | Privacy-preserving diagnostics export pattern; aligns with deny-by-default data posture |
| Setup wizard + doctor health checks | ⚙ Feature adoption | §onboarding | Directly serves non-technical onboarding invariant |
| MCP tool-name prefixing + durable background-task adaptation | 🧩 Mechanism only | §tool-integration | Namespace-collision prevention and bounded async task wrapping are reusable patterns |
| Tracing via LangSmith/Langfuse cloud | ❌ Conflict-skip | — | Ships run contents to third-party SaaS by default config choice |

## Notable absences
No deny-by-default egress gateway for agent-initiated network calls (proxies exist for sandbox traffic only); no department-scoped permission model — authz is per-user session, not per-function; no immutable audit log (receipts/checkpoints are operational, mutable); heavy standing-process architecture fundamentally at odds with ephemeral-worker model.
