# Oh My Pi (omp) — Feature Dossier
Registry: #46 | Category: 01 | Verified: 26,513 stars, MIT, last push 2026-08-22

SLOT FOCUS: LSP diagnostics as machine-verifiable evidence channel for §10.2 refine loops.

## Feature inventory
- `lsp` tool exposes diagnostics, navigation, symbols, renames, code actions, and raw LSP requests to the agent ("everything your IDE knows, the agent knows"); 14 lsp ops alongside 28 dap ops [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/README.md]
- LSP wired into every write: rename requests go through `workspace/willRenameFiles` so re-exports, barrel files, and aliased imports update *before* the file moves [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/README.md]
- Auto-detection of built-in language servers: intersects cwd `rootMarkers` presence with binary resolution checked first in project-local bins (`node_modules/.bin/`, Python venvs, Ruby binstubs, Go `bin/`) then `$PATH`; cwd-only, no ancestor walk [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/docs/lsp-config.md]
- 48 built-in servers in `defaults.json` (rust-analyzer, gopls, pyright/basedpyright, typescript-language-server, clangd, jdtls, …) plus linter-only servers (eslint, ruff, biome, rubocop) [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/docs/lsp-config.md]
- `ServerConfig` schema: `command`, `args`, `fileTypes`, `languageId`, `rootMarkers`, `initOptions`, `settings`, `disabled`, `warmupTimeoutMs`, `isLinter`, `capabilities`, `workspaceReadyTimings`; partial overrides merged onto defaults, invalid definitions ignored with warning [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/docs/lsp-config.md]
- `isLinter` flag excludes linter/formatter-only servers from type-intelligence operations — separating lint noise from compiler-grade type errors [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/docs/lsp-config.md]
- Idle shutdown (`idleTimeoutMs`) kills inactive language servers; rust-analyzer workspace-readiness polling timings tunable (`timeoutMs`/`pollMs`/`settleMs`) so diagnostics are only trusted after the index settles [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/docs/lsp-config.md]
- Config precedence chain merging `lsp.json/yaml/yml` from home dir, plugins, `~/.claude|~/.codex|~/.gemini` compat dirs, `<cwd>/.omp/`, and cwd root — highest wins, shallow per-server [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/docs/lsp-config.md]
- Hashline edit format with content-hash anchors: stale-anchor patches rejected before they corrupt anything (edit-time precondition, pairs with post-edit diagnostics) [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/README.md]
- Time-traveling stream rules: regex match aborts the stream mid-token, injects the rule as system reminder, retries from same point; injections survive compaction [REPORTED-unverified]
- Subagent fan-out (`task`) into isolated worktrees returning schema-validated typed result objects readable field-by-field (`agent://<id>/findings.0.path`) [VERIFIED-fetched:https://raw.githubusercontent.com/can1357/oh-my-pi/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Post-edit LSP diagnostics as agent-visible evidence | ⚙ Feature adoption | §10.2 refine loop evidence channel | Machine-verifiable pass/fail signal after each refine iteration: type errors are deterministic, local, and free of LLM judgment. Execution stays inside the ephemeral worker on self-hosted Linux. |
| `workspace/willRenameFiles`-aware writes | ⚙ Feature adoption | Coding dept write path | Rename correctness provable by grep-zero-residue after the move; converts a class of silent breakage into a checked operation. |
| `isLinter` split of lint vs type-intelligence servers | 🧩 Mechanism only | §10.2 evidence weighting | Two evidence tiers: style warnings are advisory, type errors block refine-loop exit. Pattern portable to any diagnostic source ranking. |
| rootMarkers + project-local binary auto-detection, idleTimeoutMs teardown | 🧩 Mechanism only | Worker lifecycle mgmt | Keeps language servers task-scoped: spawn on first touch, kill on idle/worker exit — satisfies ephemerality despite LSP servers being processes. |
| Multi-source `lsp.json` precedence chain (incl. reading `.claude/lsp.*`) | 🧩 Mechanism only | Config dept | Allow-listed config discovery with defined precedence mirrors LastHarness settings layering; inheriting other tools' configs eases migration. |
| Hashline stale-anchor rejection | 🧩 Mechanism only | Edit application | Precondition-checked patching; rejected-before-corrupt beats retry-on-broken-state. |
| Stream rules (mid-token abort + rule injection + resume) | 🧩 Mechanism only | Guardrail engine | Cheap course-correction without context tax; complements refine loops with inline enforcement. |
| Typed schema-validated subagent returns | 🧩 Mechanism only | §16.5 child delegation | Structured child output parseable by kernel — no prose parsing between siblings. |
| Advisor second-model watching every turn | ❌ Conflict-skip | — | Default routes through cloud provider APIs per turn; conflicts with local-first inference unless pinned to local models, and doubles cost. |
| Browser/computer desktop control, collab relay | ❌ Conflict-skip | — | Out of scope for LastHarness lanes; relay adds external network surface against default-deny egress. |

## Notable absences
No persistent decision log (session transcripts only, not immutable append-only audit); no department-scoped memory gating (memory is project-scoped by default, not role/dept-scoped); no deny-by-default egress gateway — `web_search` chains up to 23 providers including cloud APIs; requires Bun ≥ 1.3.14 runtime (must be vendored to honor the self-hosted boundary); diagnostics are surfaced but not persisted as structured per-refine-cycle verdict records. LSP evidence channel works only where a competent language server exists (48 built-ins cover mainstream languages, nothing for arbitrary DSLs).
