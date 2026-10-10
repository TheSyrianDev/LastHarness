# LangGraph — Feature Dossier
Registry: #66 | Category: 06 | Verified: 40,250 stars, MIT, last push 2026-08-22

SLOT FOCUS: graph checkpointing with replay/fork for recovery root-cause (§13.2).

## Feature inventory
- Checkpointer interface: `builder.compile(checkpointer=...)` persists every super-step of graph state keyed by `configurable.thread_id`; `InMemorySaver` for dev, `SqliteSaver`, `PostgresSaver`, `RedisSaver`, `MongoDBSaver`, `OracleSaver` for production; checkpointer auto-propagates to subgraphs [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/add-memory]
- `get_state_history(config)` returns the full reverse-chronological checkpoint history of a thread, each entry carrying `next` node(s) and its `checkpoint_id` config [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/use-time-travel]
- Replay: `graph.invoke(None, prior_checkpoint_config)` resumes from that checkpoint — nodes before it are not re-executed (results already saved), nodes after re-execute including LLM/API calls; replaying the final checkpoint is a no-op [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/use-time-travel]
- Fork: `graph.update_state(prior_checkpoint.config, values={...})` creates a *new* checkpoint branching from the prior point — original execution history stays intact (explicitly not a rollback); resume with `invoke(None, fork_config)`; `as_node` attribute attributes the update to a chosen node so reducers apply and successors run [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/use-time-travel]
- `interrupt()` human-in-the-loop pauses are always re-triggered during replay/fork — a forked branch can preserve earlier interrupt answers and re-ask only later ones (fork between two interrupts) [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/use-time-travel]
- Subgraph time-travel granularity control: default subgraphs inherit parent checkpointer and count as one super-step; `compile(checkpointer=True)` gives subgraphs their own checkpoint history reachable via `get_state(config, subgraphs=True)` → `tasks[0].state.config` [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/use-time-travel]
- Cross-thread long-term Store: namespaced key-value items (`store.put/search/aput`) with optional semantic-search index, injected into nodes via `Runtime`; independent of checkpoint lifecycle [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/add-memory]
- Documented failure modes + fixes: in-memory saver loses state on restart, unbounded checkpoint growth needs retention policy/pruning, thread_id length limits on Postgres [VERIFIED-fetched:https://docs.langchain.com/oss/python/langgraph/persistence]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Per-step state snapshots keyed by thread_id | ⚙ Feature adoption | §13.2 recovery substrate | Every agent step becomes an addressable, restorable point. With SqliteSaver this is local-file persistence matching LastHarness's SQLite posture. |
| `get_state_history` + replay from any checkpoint | ⚙ Feature adoption | §13.2 recovery root-cause | Post-failure: rewind to last good step and replay forward without redoing completed work — turns "what was the state when it broke?" into a queryable snapshot instead of log archaeology. |
| Fork via `update_state` (branch, history immutable) | ⚙ Feature adoption | §13.2 counterfactual analysis | Corrected-input what-if runs branch off the incident timeline while the original record remains untouched — mirrors LastHarness's immutable-decision-log + corrective-branch pattern. |
| Interrupts re-fire on replay/fork; fork-between-interrupts | 🧩 Mechanism only | §4.6 gating interplay | Any recovery mechanism must define semantics for pending approvals; LangGraph's answer (re-pause, keep earlier answers) is the right default. |
| Subgraph `checkpointer=True` granularity switch | 🧩 Mechanism only | §16.x child-run recovery | Lets deep child pipelines be individually rewound vs treated as atomic — needed so recovery doesn't re-run an entire expensive subtree. |
| Pluggable Saver backends (Postgres/Redis/Mongo/Oracle) | ❌ Conflict-skip | — | Non-SQLite backends pull standing database services into the boundary; SqliteSaver variant alone fits the self-hosted single-node posture. |
| Cross-thread semantic Store | ⚙ Feature adoption | Department-scoped gated memory (with modification) | Right shape for durable memory (namespaced, injected, searchable) but namespace policy must be department+gate enforced rather than app-defined free-form. |
| Agent Server / LangSmith deployment & tracing | ❌ Conflict-skip | — | Cloud-hosted execution and telemetry — violates self-hosted/local-first invariants wholesale. |

## Notable absences
Checkpoints capture graph state, not an audit ledger: no built-in record of *who approved what* or decision rationale — LastHarness's §13.2 still needs its own decision-log layer above checkpoints; replay is not deterministic (LLM calls re-fire and may differ), so root-cause evidence must be captured at checkpoint time, not inferred from replay; no diff visualization between checkpoints (state snapshots are opaque without tooling); no retention/archival policy shipped, only advice.
