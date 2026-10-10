# Token-Goat — Feature Dossier
Registry: #4 | Category: 02 | Verified: ~104 stars, PolyForm Noncommercial 1.0.0, last push 2026-08-22 (GitHub API 2026-08-23)

## Feature inventory
- Re-read denial + dedup: PreToolUse hook actively denies re-reads of files confirmed in the current context (`deny_reread` default on); SHA-256 fingerprinting suppresses duplicate hints; hints name previously-accessed symbols [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Surgical-read command family: `read file::Symbol`, `section doc::Heading`, `skeleton`/`outline`, `semantic "<what it does>"`, `symbol` — ~85% smaller than full reads; outputs capped ~25k tokens with narrowing-command hints [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Image shrink pipeline: screenshots re-encoded WebP/AVIF via sharp (~97% smaller vs raw PNG, ~39% vs JPEG q85); cached by path+size+mtime [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Bash-output compression: 130+ CLI filters (pytest, jest/vitest, cargo, npm/pnpm/yarn/bun, docker, kubectl/helm, terraform, git, curl, tsc, go test…) keep errors verbatim, collapse passes/progress bars, cap 1000 lines/64 KiB; compound commands wrapped per segment [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- PreCompact structured manifest: edited files + inline diffs, accessed symbols, `### MUST_PRESERVE` sealed block, Active Skills list, TODOs; capped at 500→300 tokens as context pressure rises [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Skill-overhead cut: `<!-- COMPACT_END -->` marker lets the hook serve a ~400-token compact slice instead of a 10k–65k skill body; repeat skill invocation blocked at PreToolUse unless compaction fired; direct SKILL.md reads intercepted with `skill-body <name>` hint [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Prompt-injection guard: every fetched web page scanned for attack patterns and wrapped in an untrusted-content fence before reaching the model; matched pattern names logged; one config line to disable [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Context-pressure scaling: thresholds tighten against a fixed 660k autocompact budget (large-read redirect 512 KB → ~92 KB near-full; surgical-read suggestions fire on smaller files; manifest shrinks) [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Cross-session read-dedup evidence: project-scoped SHA-256 records add compact advisories on unchanged cross-session re-reads; changed files named at startup for fresh surgical reads [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- MCP result compression: deterministic tableify + empty-value pruning + GitHub/browser field packs (only applied when ≥15% saved); full original recoverable by ID [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Output cache & recall: bash/web/MCP outputs cached by ID (`bash-output <id> --grep/--tail/--section`, `web-output <id>`); repeated monitoring commands get recall pointers instead of re-running; escalating "ran N×" warnings [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Multi-host bridges: native Claude Code hooks plus Gemini CLI/Qwen/Kimi/Codex/opencode/OpenClaw/pi adapters with documented per-platform capability losses (e.g. no context injection on opencode) [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]
- Optional detached background indexer launched via `token-goat worker start` (not started by install) [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Re-read denial w/ SHA-256 hint fingerprints | 🧩 Mechanism only | Context kernel § | Pure interception pattern; kernel-mediated read routing adopts deny-repeat semantics directly |
| Surgical-read family (symbol/section/skeleton/semantic) | ⚙ Feature adoption | Retrieval dept § | Becomes the standard read path for ephemeral workers; needs a local indexer inside the self-hosted box |
| Bash-output compression filters (failures-first) | ⚙ Feature adoption | Worker supervision § | Deterministic post-filters keep errors verbatim while collapsing noise; owned by execution dept |
| PreCompact structured manifest (MUST_PRESERVE, diffs, skills) | 🧩 Mechanism only | Gated memory dept § | Manifest shape is reusable as compaction payload into the decision log; hook wiring is host-specific |
| Skill-body COMPACT_END caching + reload blocking | 🧩 Mechanism only | Skills/plugins dept § | Companion to progressive skill loading (~400-tok slice vs 10–65k body); requires host hook parity |
| Untrusted-content fence on fetched pages | ⚙ Feature adoption | Egress gateway § | Injection scanning belongs exactly at LastHarness's default-deny egress boundary before content reaches any worker |
| Context-pressure adaptive thresholds (fixed budget denominator) | 🧩 Mechanism only | Context kernel § | Budget-relative tightening is portable to kernel-side read gating |
| Cross-session evidence records (hashes only, bodies never injected) | 🧩 Mechanism only | Gated memory dept § | Evidence-of-prior-access without content replay fits department-scoped memory hints |
| Detached background indexer daemon | ❌ Conflict-skip | — | Standing process violates ephemeral-worker invariant; re-home indexing as kernel-spawned one-shot jobs |
| PolyForm Noncommercial license on all code | ❌ Conflict-skip | — | Commercial deployment requires a paid license; adopt mechanisms/patterns, not code |

## Notable absences
Read/output-path only — no write-path or command policy engine (size heuristics, not allow-lists). Injection guard covers fetched web pages, not arbitrary tool outputs or file reads. Caches/stats are single-user, no department partitioning or access control. Node.js 22 runtime adds supply-chain surface inside the Linux boundary. Fail-open philosophy (hooks print `{}` and proceed) is the opposite of deny-by-default.
