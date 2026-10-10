# Everything Claude Code — Feature Dossier
Registry: #2 | Category: 05 | Verified: ~1.6k stars, MIT per README badge (GitHub API license field empty), last push 2026-01-23 (GitHub API 2026-08-23)

## Feature inventory
- Ships as an installable Claude Code plugin/marketplace (`/plugin marketplace add …everything-claude-code`) bundling `agents/`, `skills/`, `commands/`, `rules/`, `hooks/`, `scripts/`, `contexts/`, `mcp-configs/`; manual copy-path also documented [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- 9 role-scoped subagents with declared tools and model pinning: planner, architect, tdd-guide, code-reviewer, security-reviewer, build-error-resolver, e2e-runner, refactor-cleaner, doc-updater [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- Memory-persistence lifecycle hooks (Node.js, cross-platform): `session-start.js` loads saved context, `session-end.js` saves state, `pre-compact.js` snapshots pre-compaction [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- Continuous-learning loop: `evaluate-session.js` extracts patterns from sessions into reusable skills; `/learn` command mid-session + `continuous-learning` skill [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- Strategic compaction: `suggest-compact.js` hook + `strategic-compact` skill advise manual compaction points [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- Verification machinery: `eval-harness` + `verification-loop` skills, `/checkpoint` (save verification state), `/verify` (run verification loop) [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- Always-on rules files: security.md (no hardcoded secrets), coding-style.md (immutability, file organization), testing.md (TDD + 80% coverage requirement), git-workflow.md, agents.md (when to delegate), performance.md (model selection, context management) [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- Dynamic system-prompt injection contexts: dev.md / review.md / research.md mode files [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- Package-manager detection ladder: `CLAUDE_PACKAGE_MANAGER` env > `.claude/package-manager.json` > `packageManager` field > lockfile sniffing > global config > first available; `/setup-pm` command [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- MCP context budget doctrine: 20–30 configured, under 10 enabled per project, under 80 active tools; `disabledMcpServers` per-project disable [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]
- All hooks/scripts rewritten in Node.js for Windows/macOS/Linux + shipped test suite (`node tests/run-all.js`) [VERIFIED-fetched:https://raw.githubusercontent.com/WorldFlowAI/everything-claude-code/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Role-scoped subagents w/ tool lists + model pinning | 🧩 Mechanism only | Worker catalog § | Markdown-defined workers with restricted tool surfaces mirror ephemeral SDK workers; adopt the definition schema, not the content |
| Session-lifecycle memory hooks (start/end/pre-compact) | ⚙ Feature adoption | Gated memory dept § | Right seams for context save/load; LastHarness routes them into department-scoped gated memory + immutable decision log, not loose files |
| Continuous-learning pattern→skill extraction | ⚙ Feature adoption | Skills/plugins dept § | Pattern harvesting is valuable only behind a human promote gate (deny-by-default), which `/learn`'s manual invocation approximates |
| Verification loop + checkpoint state (/checkpoint, /verify) | ⚙ Feature adoption | Verification gate § | Persisted verification state between runs fits the propose-then-execute worker protocol |
| Always-on rules pack w/ testing/security minimums | 🧩 Mechanism only | Policy pack § | Instruction-only layer, no enforcement teeth; content patterns reusable |
| Mode contexts (dev/review/research) injection | 🧩 Mechanism only | Kernel routing § | Mode-scoped prompt assembly is a kernel concern; trivially reimplemented |
| Package-manager/config detection ladder | 🧩 Mechanism only | Onboarding § | Cascading config resolution (env>project>lockfile>global) is a proven non-technical-onboarding pattern |
| MCP enablement budget (<10 active, <80 tools) | 🧩 Mechanism only | Tool allow-list § | Operationalizes deny-by-default tool surface sizing |

## Notable absences
No execution sandbox — hooks run arbitrary shell/Node directly on host. No egress control beyond advice; MCP configs carry third-party cloud endpoints with `YOUR_*_HERE` key placeholders. Memory persistence writes unstructured files, no audit log or staleness handling. No onboarding UI (assumes Claude Code fluency). Curated config collection, not a product: GitHub API reports no license metadata despite the MIT badge.
