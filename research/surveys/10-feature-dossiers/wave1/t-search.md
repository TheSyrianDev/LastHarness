# T-Search Harness — Feature Dossier
Registry: #59 | Category: 03 | Verified: 41 stars, Apache-2.0, last push 2026-07-15 (GitHub API 2026-08-23)

## Feature inventory
- Round-based agentic-search retriever harness: session runs up to N rounds; each round starts with a **fresh context**, and only chunks the model explicitly saves via `save_and_advance` carry forward [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Three-tool loop surface: `search_corpus`, `save_and_advance`, `finalize_ranking`; single entry point `agent.retrieve(query)` → ranked `RetrievalResult` [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Bring-your-own retrieval backend: one-method structural `SearchClient` Protocol (`search(query, top_k) -> JSON string` of `{docid, snippet, score}`) — in-process wrapper or HTTP adapter, no subclassing; harness ships no index [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Snippet-only hand-off contract: no full-document fetch; fixed output fields (`chunk_id`, `text`, `score`, `rank`, `retrieval_query`) for downstream consumers [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Token-budget governance knobs: `budget_tokens` per-round context budget, `hard_lock_ratio` (0.75) locks searching at budget threshold leaving only save/finalize, `min_searches_before_save`, `saved_soft_cap` [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Degenerate-behavior guards with force-termination: `free_text_guard_limit` (consecutive no-tool turns), `locked_search_streak_limit`, `max_turns_per_round` safety cap, finalize rejection when too many concepts unresolved, zero-new-chunk state warnings [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Failure surfacing instead of crashes: search errors retried up to 3× with backoff then surfaced to the model as a tool error while the session continues; malformed JSON handled the same way [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Runs against any OpenAI-compatible endpoint (documented against local vLLM serving T-Search); fully typed (`py.typed`), Python ≥3.10, Poetry/pip installable [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Config via pydantic-settings: per-field defaults valid out of the box, overridable per field, from YAML mapping, or env vars (`RETRIEVER_AGENT_<FIELD>`) [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]
- Per-run telemetry and raw transcript capture (`rounds_completed`, `tool_call_counts`, `round_summaries`, `messages`) with explicit prompt-leak warning against blindly logging `.to_dict()` [VERIFIED-fetched:https://raw.githubusercontent.com/turbo-llm/t-search-harness/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Fresh-context rounds + explicit save-carry-forward | ⚙ Feature adoption | Search/Data dept retrieval worker | Context hygiene pattern keeps workers bounded and ephemeral; saved-set is scoped working memory |
| One-method SearchClient protocol (BYO backend) | 🧩 Mechanism only | Kernel routing § | Kernel-mediated single choke point between agent and corpus; no peer-to-peer, trivially allow-listable |
| Snippet-only hand-off + fixed result schema | 🧩 Mechanism only | Search/Data § | Deterministic contract between retrieve and downstream generator; prevents context flooding |
| Budget hard-lock ratio + min-saves + soft caps | ⚙ Feature adoption | Worker resource governor | Resource-bounded autonomy matches deny-by-default limits; prevents runaway spend on local inference too |
| Degenerate-loop guards / force-terminate | ⚙ Feature adoption | Worker supervision § | Loop breakers are exactly what kernel-side watchdogs need for unattended runs |
| Error-as-tool-feedback retry policy | 🧩 Mechanism only | Worker error handling § | Failures become model-visible events instead of crashes; auditable in decision log |
| OpenAI-compatible local endpoint support (vLLM) | ⚙ Feature adoption | Local inference dept | Aligns 1:1 with local-first inference invariant |
| Telemetry + transcript with leak warning | ⚙ Feature adoption | Observability § | Structured run records feed the immutable decision log; leak warning is good log-hygiene precedent |
| Prompts/tool schemas ARE the operating point (tuned for T-Search model) | 🧩 Mechanism only | Model packs § | Confirms prompts+schemas must be versioned together with hardware/model tier |

## Notable absences
No persistence layer or service mode — single-query sessions only, nothing standing (good for invariants, but LastHarness needs a wrapper). No UI, no auth, no multi-tenant scoping. No reranker/generator included — ranking hand-off is deliberately terminal. Behavior tied to T-Search-trained prompts/thresholds; swapping models requires re-tuning. No egress controls on the HTTP search client.
