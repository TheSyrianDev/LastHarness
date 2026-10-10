# Flowise — Feature Dossier
Registry: #30 | Category: 06 | Verified: 55,388★, NOASSERTION (README claims Apache 2.0 via LICENSE.md), last push 2026-08-13 (repo-verify.txt 2026-08-24)

Drag-and-drop visual LLM workflow builder (Assistant / Chatflow / Agentflow canvases). **The GitHub repo is archived** — the README's top banner defers to a "Future of Flowise" discussion — so this dossier treats it strictly as a pattern donor, not an adoption candidate.

## Feature inventory
- Three visual builders: Assistant (beginner chat agent with tool use + RAG over uploaded files), Chatflow (single-agent flows with Graph RAG, reranker, retriever techniques), and Agentflow V2 (superset for chat assistants, single- and multi-agent systems, complex workflow orchestration) [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Orchestration capability set: visual editor with expression support, custom code nodes, branching/looping/routing logic; open-source and proprietary model support [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Data ingestion/integration: 100+ connectors across sources, tools, vector databases, and memories [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Monitoring: execution logs, visual debugging of flow runs, external log streaming [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Deployment story: self-hosted options explicitly including air-gapped deploy; Node.js ≥ 20 runtime (`npx flowise start` on localhost:3000) or docker compose; monorepo of server (Node backend), ui (React), components (third-party node integrations), auto-generated swagger API docs [VERIFIED-fetched:https://raw.githubusercontent.com/FlowiseAI/Flowise/main/README.md]
- MCP integration: MCP client/server nodes with tool listing, SSE transport, auth support [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Safety & control: input moderation and output post-processing nodes; security controls listing RBAC, SSO, encrypted credentials, secret managers, rate limits, restricted domains [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Evaluations: datasets, evaluators, and evaluations as first-class objects [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Delivery surfaces: REST API, JS/Python SDK, CLI, embeddable shareable chat widget, template marketplace with reusable components; Teams & Workspaces multi-tenancy [VERIFIED-fetched:https://docs.flowiseai.com/readme.md]
- Flowise Cloud: vendor-hosted SaaS edition alongside self-host [VERIFIED-fetched:https://raw.githubusercontent.com/FlowiseAI/Flowise/main/README.md]
- Repository archived: banner "Flowise has been archived. Refer to Future of Flowise" linking discussion #6727 [VERIFIED-fetched:https://raw.githubusercontent.com/FlowiseAI/Flowise/main/README.md]
- License discrepancy: repo-verify.txt records NOASSERTION while the README states source is under Apache License Version 2.0 via LICENSE.md [REPORTED-unverified]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Visual drag-and-drop DAG authoring of agent workflows | 🧩 Pattern only | Owner-console pipeline editor candidate (R7 adjacency) | Authoring a visual graph that the KERNEL then validates and dispatches is compatible; execution must never live in the canvas server |
| Agentflow V2 multi-agent orchestration executed on a standing Flowise server | ❌ Violates kernel-mediated routing (§4.4) + ephemeral worker lock | — | A resident orchestration engine routing peer-to-peer between flow nodes is exactly what the kernel replaces; WHERE execution happens = inside a standing third-party runtime |
| 100+ third-party connectors/tools/vector-DB/memory integrations | ❌ Violates default-deny egress gateway | — | Each connector is an unscoped outbound channel; only a rebuilt allow-listed adapter subset could ever pass the gateway |
| Execution logs + visual run debugging | ⚙ Capability owner console owns | R7 owner-console trace view §13.4 | Visual replay of dispatched runs over local logs is an owner-visibility win; feed from the immutable decision log, not its own DB |
| Air-gapped/self-hosted deployment option | 🧩 Pattern only | Boundary feasibility precedent | Vendor-documented offline mode confirms this class of tool can live fully inside one Linux boundary |
| Evaluations (datasets/evaluators) | ⚙ Capability Verification owns | §11.3 verification-suite adjacency | Dataset+evaluator object model transfers to LastHarness's verification suites as authored, versioned evidence artifacts |
| Input moderation / output post-processing nodes | 🧩 Pattern only | §16.6 injection-scan pairing | In/out screening as explicit pipeline stages matches the adopted scan doctrine |
| RBAC/SSO/Teams & Workspaces | 🧩 Pattern only | Mostly N/A single-owner | Multi-tenant governance machinery exceeds LastHarness needs; encrypted-credential storage habit worth keeping |
| Embeddable chat widget + template marketplace | ❌ Out of scope | — | Public-facing delivery surfaces and community templates imply external content ingestion with no provenance gate |
| Flowise Cloud SaaS edition | ❌ Violates ONE self-hosted boundary (no cloud execution of agent workloads) | — | Cloud-hosted flow execution is the direct negation of the invariant |

## Notable absences
No maintenance future: the archive banner means bugfixes and node updates have stopped except per the fork/community discussion — a hard adoption blocker. No sandboxing model for custom-code nodes (arbitrary JS executes server-side with no confinement story documented). No local-inference guidance in either source ("open-source models" is as specific as it gets; nothing about llama.cpp/Ollama wiring). License ambiguity: verify-file NOASSERTION vs README Apache claim needs resolution before any code reuse. No egress allow-listing, no audit-log immutability, no provenance on template/marketplace components.
