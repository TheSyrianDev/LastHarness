# LangChain — Feature Dossier
Registry: #65 | Category: 06 | Verified: 144,892 stars, MIT, pushed 2026-08-24 (GitHub API 2026-08-24); langchain 1.3.16 on PyPI, requires Python ≥3.10 (PyPI JSON API, fetched 2026-08-24)

Skip-confirmation dossier. Recorded disposition: framework switch rejected §16.2. Task asked for a brief confirmation of current framing: LangChain remains on the **1.x line** (1.3.16; no 2.x), and its framing is now fully agent-first — the README's headline is "The agent engineering platform" with agents as the primary artifact, chains demoted to one abstraction among many.

## Feature inventory
- **Repositioning**: "The agent engineering platform"; "LangChain is a framework for building agents and LLM-powered applications" — interoperable components + third-party integrations [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/langchain/master/README.md]
- **Current version**: 1.3.16, Python `>=3.10,<4.0` [VERIFIED-fetched:https://pypi.org/pypi/langchain/json]
- **Model interoperability as the core pitch**: `init_chat_model("openai:gpt-5.5")` quickstart; "Swap models in and out" as abstractions evolve [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/langchain/master/README.md]
- **Ecosystem steering**: Deep Agents ("higher-level package built on LangChain" for planning/subagents/filesystem) recommended first; LangGraph for low-level orchestration; LangSmith for evals/observability; LangSmith Deployment for "long-running, stateful workflows" [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/langchain/master/README.md]
- JS/TS sibling library maintained in parallel (LangChain.js) [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/langchain/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Framework adoption as LastHarness's agent substrate | ❌ Conflict-skip | — | Confirmed unchanged: §16.2 locks every worker to the Claude Agent SDK; LangChain-as-runtime repeats the rejected-alternative ruling (Google ADK family), and wave-2's deep-agents skeptic fix already downgraded harness-as-runtime to pattern-donor. |
| Agent-first platform pivot (v1.x) | 🧩 Mechanism only | Market validation for §16.2's agent-native architecture | The largest framework in the space reorganized its entire identity around agents — convergent evidence that LastHarness's agent/workers/kernel framing is the industry direction, not a bet against it. |
| Model-interoperability layer (`init_chat_model`) | 🧩 Mechanism only | Already covered by llama-swap aliases §16.1 | Same indirection at a different layer; LastHarness swaps models via alias routing without a client-side abstraction library. |
| LangSmith observability/deployment pull | ❌ Conflict-skip | — | SaaS evals/observability and hosted long-running deployment are cloud execution/egress surfaces; LastHarness adopted the LangFuse-shaped local span schema in §16.6 Output 3 instead. |
| Deep Agents / LangGraph sub-ecosystems | 🧩 Mechanism only | Cross-reference: wave1/langgraph.md, wave2/deep-agents.md | Both already dossiered separately with their own extractions; nothing new at README level here. |

## Notable absences
Brief by design (skip-tier). At README level there is no permission/approval story, no memory lifecycle, no egress or audit concept, no local-inference guidance — those live downstream in integrations/LangGraph docs already covered by prior dossiers. Nothing observed that would reopen the §16.2 rejection.
