# Context Mode — Feature Dossier
Registry: #11 | Category: 02 | Verified: ~20.1k stars, ELv2 (Elastic License 2.0; GitHub API NOASSERTION), last push 2026-08-22 (GitHub API 2026-08-23)

## Feature inventory
- Sandbox execution tools: `ctx_execute` / `ctx_execute_file` / `ctx_batch_execute` run scripts against the workspace so only printed results enter context (claimed 315 KB → 5.4 KB, 98% reduction; Playwright snapshot 56 KB and 20 GitHub issues 59 KB cited as raw costs) [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- "Think in Code" paradigm: agent programs the analysis instead of hauling raw data through context (one `ctx_execute` script replacing 47 Reads, 700 KB → 3.6 KB); mandatory across all supported clients [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Session continuity store: every file edit, git operation, task, error, and user decision tracked in SQLite; on compaction events are FTS5-indexed and retrieved via BM25 search rather than dumped back into context; resume snapshots rebuilt at session start [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Default-ephemeral retention: if you don't `--continue`, previous session data is deleted immediately — fresh session = clean slate [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Routing enforcement hooks per platform: PreToolUse deny/block on flood-prone tools (Bash/Read/Grep/WebFetch/Task/MCP wildcards), PostToolUse capture, UserPromptSubmit/PreCompact/SessionStart/Stop wiring; fails open on hosts lacking hook features [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Runtime SessionStart routing-block injection (Claude Code writes no project files); AGENTS.md/rules-file fallback for hookless platforms with ~60% compliance vs ~98% with hooks [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- 11 MCP tools: six sandbox (`ctx_execute`, `ctx_execute_file`, `ctx_batch_execute`, `ctx_index`, `ctx_search`, `ctx_fetch_and_index`) + five meta (`ctx_stats`, `ctx_doctor`, `ctx_upgrade`, `ctx_purge`, `ctx_insight`) [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Persistent knowledge base: `ctx_index`/`ctx_search` over FTS5 for local files/directories; `ctx_purge` wipes it [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Savings telemetry: `ctx_stats` per-tool token breakdown + optional statusline showing $ saved this session / across sessions / % efficient [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Hosted Insight dashboard (context-mode.com/insight) for org-level AI-assisted-engineering analytics [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- Explicit no-prose-enforcement stance: routing block governs where data goes, not how the model talks, citing benchmark degradation from aggressive brevity prompts [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]
- 17-client adapter matrix incl. native OpenClaw gateway plugin (8 gateway hooks) and Pi extension with full lifecycle events [VERIFIED-fetched:https://raw.githubusercontent.com/mksglu/context-mode/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Tool-output sandboxing (script executes locally, only result enters context) | ⚙ Feature adoption | Execution / kernel routing § | Inverts the data flow — workers get answers, not payloads; core mechanism for keeping ephemeral workers lean |
| Think-in-Code mandate | 🧩 Mechanism only | Worker protocol § | Prompt+tool-shape convention with zero infrastructure; fold into worker system prompts |
| SQLite event log + FTS5/BM25 index-not-dump compaction | 🧩 Mechanism only | Decision-log / gated memory § | Retrieval-at-compaction beats summary lossiness; pattern ports to kernel compaction protocol |
| Delete-unless-continued session retention | ⚙ Feature adoption | Gated memory dept § | Default-ephemeral retention is exactly the right posture; persistence becomes an explicit act |
| PreToolUse deny/block flood routing w/ fail-open caveat | 🧩 Mechanism only | Kernel hook points § | Deny-by-default enforcement point for output-size policy — but LastHarness must fail closed |
| Persistent FTS5 knowledge base (ctx_index/ctx_purge) | 🧩 Mechanism only | Retrieval dept § | Useful capability; needs department scoping before adoption |
| Hosted Insight dashboard | ❌ Conflict-skip | — | Third-party cloud analytics violates self-hosted boundary and default-deny egress |
| Statusline savings meter | 🧩 Mechanism only | Console UI § | Cheap observability nicety |

## Notable absences
No content-level policy engine — decisions are size-heuristic based, not allow-list based. No department-scoped permissions or multi-tenant separation on the knowledge base or stats. ELv2 license restricts redistribution (adopt mechanisms, not code). Session DB is single-host by design (fine for one-box, no HA story). Hook coverage is uneven across the 17 clients (Codex PreToolUse supports deny-only today).
