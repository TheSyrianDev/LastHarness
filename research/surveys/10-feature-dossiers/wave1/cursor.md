# Cursor — Feature Dossier
Registry: #54 | Category: 01 | Verified: ~33.2k stars on tracker repo (GitHub API 2026-08-23), no OSS license (closed source; repo is issue-tracker only), last push 2026-05-12 — product facts sourced from cursor.com docs, not code

## Feature inventory
- Cursor is a closed-source commercial coding agent (editor + agent); the `cursor/cursor` GitHub repo serves only as bug/feature forum and security intake [VERIFIED-fetched:https://raw.githubusercontent.com/cursor/cursor/main/README.md]
- Agent surface: Agents Window, Agent Review, Plan Mode (scoped planning before build), Debug Mode, Design Mode, prompting guides [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- Customization stack: Plugins, Rules (project behavior config), Skills (on-demand capability packages), Subagents, Hooks, MCP servers — added "from one place" [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- Cloud Agents: remote/cloud execution with Builds, Automations (scheduled/recurring), Bugbot (bug finding), Security Agents, PR Routing & Approval flows, and Mobile access [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- CLI with Shell Mode, ACP support, Headless/CI mode for pipelines [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- SDKs: TypeScript, Python, and a Bridge API for programmatic control [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- Integrations: GitHub, GitLab, Azure DevOps, Bitbucket, JetBrains, Xcode, Slack, Microsoft Teams, Jira, Linear, Notion, deeplinks [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- Multi-model routing incl. in-house Composer 2.5 plus Anthropic/Google/OpenAI/xAI models with per-model context windows [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- "Origin": built-in git hosting / GitHub mirroring, PR browsing & search inside the product [VERIFIED-fetched:https://cursor.com/docs/get-started/concepts]
- Codebase understanding/semantic search, review-before-merge diffs and checks [REPORTED-unverified — linked from fetched docs index but page content not retrieved]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Plan Mode → execute split (scope, clarify, then build) | 🧩 Mechanism only | Kernel worker protocol § | Two-phase propose/execute is adoptable regardless of vendor |
| Rules-as-config + Skills folders + Hooks + Subagents | 🧩 Mechanism only | Skills/plugins § | Same declarative-extension pattern as open standards; validates direction |
| Agent Review / diff-inspect-before-merge | 🧩 Mechanism only | Approval gate § | Human checkpoint over machine edits is an approval-gate pattern |
| Cloud Agents, Automations, Bugbot (cloud execution) | ❌ Conflict-skip | — | Execution happens in Cursor's cloud; violates self-hosted/local-first boundary and standing-process ban |
| Origin git hosting + mirror-GitHub | ❌ Conflict-skip | — | Whole business function LastHarness would run locally via its own VCS dept; also proprietary |
| SDK/Bridge programmatic control | ⚙ Feature adoption | Orchestration § | Local control API shape worth mirroring for kernel→worker commands |
| Slack/Jira/Linear integration surfaces | ❌ Conflict-skip | — | Standing third-party SaaS connectors conflict with ephemeral-worker + default-deny egress invariants |
| In-house model tier (Composer) alongside external models | ⚙ Feature adoption | Model packs § | Supports hardware-tiered local pack strategy: own models first, externals optional |

## Notable absences
No source available to audit execution path, telemetry, or data handling (closed source; repo README explicitly routes everything to web properties). No self-host option. No local-only mode documented for cloud-agent features. Privacy/security posture exists only as marketing/docs claims — cannot be verified from code.

THIN-SOURCE note: repo itself is thin by design; feature list above leans on official docs index (fetched). Anything behind unvisited doc pages remains REPORTED-unverified.
