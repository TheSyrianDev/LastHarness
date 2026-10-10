# TencentDB Agent Memory — Feature Dossier
Registry: #24 | Category: 02 | Verified: ~23.8k stars, MIT per README badge (GitHub API license NOASSERTION), last push 2026-08-15 (GitHub API 2026-08-23). Note: GitHub serves this repo under the Tencent org (TencentCloud redirects).

## Feature inventory
- Layered long-term memory pyramid L0 Conversation (raw dialogue) → L1 Atom (atomic facts) → L2 Scenario (scene blocks) → L3 Persona (user profile); persona carries daily preferences, system drills to atoms only when details matter [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Symbolic short-term memory: verbose tool logs offloaded to `refs/*.md`; task state condensed into a Mermaid canvas with `node_id`s; agent reasons over the few-hundred-token canvas and greps `node_id`/`result_ref` to recover full raw text on demand [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Lossless drill-down chain guarantee: top symbol (Persona/canvas) → mid index (Scenario/jsonl) → bottom raw text (L0/refs); no irreversible compression; deterministic path from abstraction back to evidence [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- White-box artifacts on disk under `~/.openclaw/memory-tdai/`: persona.md, scenario Markdown blocks, Mermaid canvases — human-inspectable at every layer [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Hybrid retrieval: BM25 + vector + RRF fusion (`keyword` / `embedding` / `hybrid` strategies); jieba zh tokenizer option; recall budgets (maxResults, maxCharsPerMemory, maxTotalRecallChars) and timeout-skip-injection [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Local zero-config backend SQLite + sqlite-vec default (Tencent Cloud Vector DB backend optional) [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Agent-facing tools `tdai_memory_search` / `tdai_conversation_search` [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Automated capture/extraction pipeline with tuning knobs: L1 extraction every N conversations with warm-up doubling (1→2→4…), idle-timeout trigger, L2 min interval, vector dedup/conflict detection, persona generation every N memories, per-session extraction caps, agent-exclusion globs, L0/L1 retention days [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Opt-in short-term compression offload (≥0.3.4) via OpenClaw `contextEngine` plugin slot + runtime patch script for after-tool-call messages [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Hermes Gateway: standing HTTP service on :8420 exposing capture/search/recall endpoints; opt-in Bearer apiKey (constant-time compare) and CORS allow-list, both default OFF; loud startup warnings when unset or bound non-loopback without a key [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Remote OpenAI-compatible embedding service option (incl. BGE-M3 `sendDimensions:false` workaround) and standalone LLM mode bypassing host model [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]
- Published benchmarks with OpenClaw over long-horizon sessions: WideSearch success 33%→50% (+51.52% rel) at −61.38% tokens; SWE-bench 58.4%→64.2%; PersonaMem long-term accuracy 48%→76% [VERIFIED-fetched:https://raw.githubusercontent.com/TencentCloud/tencentdb-agent-memory/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| L0→L3 layered distillation (conversation→atom→scenario→persona) | ⚙ Feature adoption | Gated memory dept § | The strongest shipped design for department memory hubs; must be re-scoped per-department with promotion gates rather than one global persona |
| Mermaid canvas + node_id offload w/ light injection | 🧩 Mechanism only | Context kernel § | Traceable context-offload pattern for long tasks; canvas format is model-parseable and human-readable |
| White-box markdown memory artifacts | 🧩 Mechanism only | Decision-log / observability § | Inspectable-by-default layers support audit without DB spelunking |
| Hybrid BM25+vector RRF recall w/ char budgets | 🧩 Mechanism only | Retrieval dept § | Standard fusion recipe; sqlite-vec variant fits local-first constraint |
| Drill-down traceability chain (no irreversible compression) | ⚙ Feature adoption | Gated memory / decision-log § | "Lower layers preserve evidence, upper layers preserve structure" is the correct contract for durable memory over an immutable log |
| Automated always-on capture pipeline (opt-out-everything defaults) | 🧩 Mechanism only | Memory capture policy § | Pipeline mechanics useful, but defaults invert deny-by-default — LastHarness needs capture allow-lists |
| Standing Gateway HTTP process (:8420 inbox endpoints) | ❌ Conflict-skip | — | Standing process with request inboxes violates ephemeral-worker invariant even though auth is opt-in |
| Remote embedding/LLM API as configured path | ❌ Conflict-skip (as-defaulted) | — | Violates local-first inference unless swapped to local embedder/model packs |

## Notable absences
No RBAC or department-level access separation — the "team hub" claim rests on shared installation, not permissioning. Capture has no allow-list (everything is captured by default within enabled agents). Gateway ships open-loopback with auth off by default (warn-only). Roadmap items unshipped: portable cross-agent memory export/import, automatic skill generation from traces, observability dashboard.
