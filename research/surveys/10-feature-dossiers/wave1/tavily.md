# Tavily (tavily-python) — Feature Dossier
Registry: #95 | Category: 03 | Verified: 1,369 stars, MIT, last push 2026-07-30

SLOT FOCUS: cleaned-extract response shape to benchmark SearXNG wrapping against (§14.3).

## Feature inventory
- `search()` web search endpoint; keyless mode runs `search()`/`extract()` with no API key against the public Tavily API, rate-limited, with structured limit exception (`code`, `window`, `retry_after_seconds`, `next_actions`) [VERIFIED-fetched:https://raw.githubusercontent.com/tavily-ai/tavily-python/master/README.md]
- Extract request shape (`POST /extract`): `urls` string or array (max 20), optional `query` for relevance-reranking of chunks, `chunks_per_source` 1–5 (chunks ≤500 chars joined with `[...]` into `raw_content`), `extract_depth` basic|advanced (advanced pulls tables/embedded content at 2× credits), `format` markdown|text, `timeout` 1.0–60.0s [VERIFIED-fetched:https://docs.tavily.com/documentation/api-reference/endpoint/extract]
- Extract response schema: `{results: [{url, raw_content, images[], favicon}], failed_results: [{url, error}], response_time: float, usage: {credits}, request_id: uuid}` — successes and failures in separate arrays so partial failure is structured, not exceptional [VERIFIED-fetched:https://docs.tavily.com/documentation/api-reference/endpoint/extract]
- `get_search_context()` one-line RAG context-string builder and `qna_search()` direct-answer helper layered over search [VERIFIED-fetched:https://raw.githubusercontent.com/tavily-ai/tavily-python/master/README.md]
- `crawl()` (invite-only) and `map()` site traversal endpoints with `instructions` steering, `max_depth`, `limit` [VERIFIED-fetched:https://raw.githubusercontent.com/tavily-ai/tavily-python/master/README.md]
- Async `research()`: create task → poll by `request_id` via `get_research()`; report carries `status`, `content`, `sources[]`; streaming variant [VERIFIED-fetched:https://raw.githubusercontent.com/tavily-ai/tavily-python/master/README.md]
- Custom session/gateway injection: pass your own `requests.Session` or `httpx.AsyncClient` plus `api_base_url` override — auth, proxies, headers handled by caller's gateway; SDK never closes externally-provided sessions [VERIFIED-fetched:https://raw.githubusercontent.com/tavily-ai/tavily-python/master/README.md]
- Optional attribution headers (`X-Project-ID`, `X-Session-Id`, `X-Human-Id` hashed server-side, `X-Client-Name`) set per client or per call [VERIFIED-fetched:https://raw.githubusercontent.com/tavily-ai/tavily-python/master/README.md]

## Per-feature verdict
| Feature | Type | 🧩/⚙ landing | Rationale |
|---|---|---|---|
| Extract response schema (`results` / `failed_results` / `response_time` / `request_id`) | 🧩 Mechanism only | §14.3 SearXNG wrapper contract | This is the de-facto shape downstream consumers expect from "cleaned extract". LastHarness's local SearXNG wrapper should emit this exact schema so backends stay swappable and benchmark comparisons are apples-to-apples. |
| `failed_results[]` with per-URL error strings | ⚙ Feature adoption | §14.3 fetch-result normalization | Structured partial failure beats all-or-nothing exceptions; each dead URL returns *why* inside a 200 response. |
| `query` + `chunks_per_source` chunk reranking (≤500 chars, `[...]` joins) | 🧩 Mechanism only | §14.3 content budgeting | Query-conditioned snippet budgeting is the pattern to replicate locally when full-page raw_content would blow context. |
| `format: markdown\|text` + `extract_depth: basic\|advanced` | 🧩 Mechanism only | Extraction pipeline tuning | Two-axis quality/cost control (depth × format) is a clean parameterization for LastHarness's own extractor tiers. |
| Gateway/session injection with `api_base_url` override | 🧩 Mechanism only | Egress gateway compatibility | Proof the whole client surface can sit behind a caller-controlled proxy — exactly how LastHarness would front any search provider through its default-deny egress gateway. |
| Keyless public mode | ❌ Conflict-skip | — | Still executes on api.tavily.com anonymously; third-party cloud execution regardless of key absence. |
| `search`/`crawl`/`research` cloud endpoints | ❌ Conflict-skip | — | All inference/extraction happens on Tavily's cloud; violates local-first and default-deny egress. Benchmark reference only — never wired live. |

## Notable absences
No self-hosted deployment path at all — the cleaning pipeline is closed, so extraction quality cannot be audited or reproduced locally (which is precisely why it serves as a *benchmark target* rather than a component); no PII scrubbing guarantees documented; credit metering is opaque server-side (`usage.credits` may read 0 below batch thresholds); no offline/batch replay of stored results.
