# Langfuse — Feature Dossier
Registry: #64 | Category: 06 | Verified: ~33.6k stars, MIT (except `ee/` folders; GitHub shows NOASSERTION), last push 2026-08-22 (GitHub API 2026-08-23)

## Feature inventory
- LLM application observability: SDK/API instrumentation ingesting traces of LLM calls, retrieval, embeddings, and agent actions; inspection and debugging of complex logs grouped into user sessions [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Prompt management: centrally manage, version-control, and collaboratively iterate prompts; server- and client-side caching so iteration adds no latency [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Evaluations: LLM-as-a-judge, code evaluators, user feedback collection, manual labeling queues, custom evaluation pipelines via APIs/SDKs [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Datasets for test sets/benchmarks with dataset runs, pre-deployment testing, structured experiments, LangChain/LlamaIndex integration [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- LLM Playground for testing prompt/model configurations, jump-to-playground directly from a bad trace [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Comprehensive public API: OpenAPI spec, Postman collection, typed Python + JS/TS SDKs; used to build bespoke LLMOps workflows [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Self-hosting paths: docker compose local ("5 minutes"), single-VM compose, Kubernetes Helm charts, Terraform templates for AWS/Azure/GCP; built on ClickHouse; Dockerfiles in-repo [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Managed Langfuse Cloud alternative (EU/US regions) alongside self-host [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Integration matrix: OpenAI SDK drop-in replacement, LangChain, LlamaIndex, Haystack, LiteLLM, Vercel AI SDK, Mastra, Ollama (local models), plus 18+ community packages (AutoGen, CrewAI, Flowise, OpenWebUI, …) [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]
- Project-scoped API credentials: per-project secret/public key pairs (`sk-lf-…`/`pk-lf-…`) configured via env, `@observe()` decorator tracing [VERIFIED-fetched:https://raw.githubusercontent.com/langfuse/langfuse/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Trace ingestion + session-level inspection UI | 🏛 Department candidate | Observability dept | Whole business function (see what every worker did); self-hostable on Linux box satisfies boundary; execution stays local |
| Versioned prompt management w/ caching | ⚙ Feature adoption | Memory dept (gated) / Model packs | Prompt versions are department-scoped artifacts; cache design avoids latency tax on local inference |
| Evals: judges, code evaluators, human labeling | 🏛 Department candidate | Eval/QA dept | Continuous quality loop over worker outputs is a full function LastHarness needs; runs locally |
| Datasets & experiment runs | ⚙ Feature adoption | Eval/QA dept | Regression harness for model-pack changes across hardware tiers |
| Playground w/ trace→playground jump | ⚙ Feature adoption | Observability → Ops hand-off | Debug loop from immutable log entry to safe re-run is a strong ops pattern |
| OpenAPI + typed SDKs | 🧩 Mechanism only | Kernel API surface § | Contract-first API style to imitate, not a dependency |
| Self-host docker-compose path | ⚙ Feature adoption | Infra dept | Single-box deployment matches self-hosted Linux boundary; K8s/Helm/Terraform paths are out of scope for one-box |
| Langfuse Cloud (SaaS) option | ❌ Conflict-skip | — | Third-party cloud execution/storage violates self-hosted + default-deny egress invariants; ignore |
| Ollama/local-model integrations | ⚙ Feature adoption | Local inference dept | Tracing local models keeps observability inside the box |

## Notable absences
No deny-by-default egress or network policy concepts — assumes it can be reached by any instrumented app. Heavy storage stack (ClickHouse + Postgres + Redis) vs LastHarness's immutable SQLite decision log — adopt as separate observability store, never as the kernel log. No per-department memory gating or approval workflows. No hardware-tier awareness or model packing. Onboarding still targets developers (env keys, decorators), not non-technical users.
