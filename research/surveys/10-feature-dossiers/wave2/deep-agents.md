# Deep Agents — Feature Dossier
Registry: #63 | Category: 06 | Verified: 28,330★, MIT, last push 2026-08-24 (repo-verify.txt 2026-08-24)

LangChain's "batteries-included agent harness": an opinionated, extensible library over LangGraph bundling sub-agents, pluggable filesystem, context management, HITL gates, skills, and memory. As a LIBRARY it is the closest-to-compliant wave-2 entry — it can run on llama.cpp/vLLM inside the boundary and its security doctrine matches LastHarness's verbatim; its ecosystem pull (LangSmith cloud) is the conflict.

## Feature inventory
- Bundled harness features: sub-agents (delegate tasks to agents with ISOLATED context windows); filesystem (read/write/edit/search over pluggable local, sandboxed, or remote backends); context management (summarize long threads, offload tool outputs to disk); shell access in a sandbox of choice; persistent memory via pluggable state/store backends for cross-session recall [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- Human-in-the-loop: approve, edit, or reject tool calls BEFORE they run [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- Skills: reusable behaviors the agent loads on demand; tools: bring-your-own functions or any MCP server [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- `create_deep_agent(model, tools, system_prompt)` minimal API; every piece overridable/replaceable without forking; layered docs position: LangGraph runtime → `create_agent` minimal harness → Deep Agents opinionated harness; any LangGraph CompiledStateGraph can be plugged in as a sub-agent [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- Model-agnostic including SELF-HOSTED models via Ollama, vLLM, llama.cpp — anything with tool calling works [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- Security doctrine stated verbatim: "trust the LLM" model — the agent can do anything its tools allow; enforce boundaries at the TOOL/SANDBOX level, not by expecting the model to self-police [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- Production posture: built on LangGraph streaming/persistence/checkpointing; first-class tracing/evaluation/deployment pushed via LangSmith (SaaS) [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- Deep Agents Code: pre-built terminal coding agent "similar to Claude Code", powered by any LLM, installed via curl script (`dcode`) [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- JS/TS twin library deepagents.js with feature parity ambitions [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]
- Acknowledged lineage: explicitly inspired by Claude Code, an attempt to generalize what makes it general-purpose [VERIFIED-fetched:https://raw.githubusercontent.com/langchain-ai/deepagents/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Sub-agents with isolated context windows + delegation | 🧩 Pure architecture pattern | Ephemeral worker composition §16.6 | Matches adopted dispatch model: each worker gets a bounded brief, no shared live state; deepagents is a reference implementation of the shape already decided |
| Pluggable filesystem incl. tool-output offload to disk + thread summarization | 🧩 Pattern only | Context-management pattern §6.3/§16.6 | Offload-to-scratch-FS keeps worker contexts bounded; backends must be pinned to local paths inside the boundary |
| Human-in-the-loop approve/edit/reject BEFORE execution | 🧩 Pattern only | Deny-by-default gate UX §13.4 | Same lever as kernel gating; invert the default (LastHarness denies first and requires allow-list membership rather than trusting then approving) |
| "Enforce boundaries at tool/sandbox level, not model self-policing" doctrine | 🧩 Pure architecture pattern | Restates deny-by-default invariant | Quote-worthy principle alignment from a 28k★ project — useful citation when defending the invariant |
| Runs on self-hosted Ollama/vLLM/llama.cpp | ⚙ Capability owned by an existing department | Candidate worker-harness library inside boundary | §16.6 input-6 / role instructions | SKEPTIC FIX: harness-as-runtime violates §16.2 SDK lock (jcode precedent, ADK rejection) — value is the isolated-context subagents + tool-output offload PATTERN only; runtime stays Claude Agent SDK |
| Skills loaded on demand + BYO tools/MCP | ⚙ Capability Information dept owns | Skill registry w/ allow-lists §14.5 | Mechanism fits, but ingestion needs the deny-by-default inversion: skills/MCP servers enter only via reviewed allow-lists |
| Pluggable REMOTE filesystem/memory backends | ❌ Conditional egress hazard | Local-only backend pinning | Remote backends are an invitation to leak workloads off-boundary; acceptable ONLY if the config surface is locked to loopback/local paths |
| LangSmith tracing/eval/deployment integration as first-class path | ❌ Violates default-deny egress gateway | Replace with local decision-log traces | Telemetry-by-default flows to vendor SaaS; LastHarness observability comes from its immutable SQLite log instead |
| Deep Agents Code prebuilt personal coding agent | ❌ Wrapped-harness product form | Pattern source only | A packaged always-ready coding agent is the standing-assistant shape LastHarness excludes; the underlying library composition remains usable |
| LangGraph persistence/checkpointing under the harness | 🧩 Pattern only | Kernel tick/resume semantics | Checkpoint-resume maps onto kernel-managed worker restarts after teardown |

## Notable absences
No built-in allow-listing machinery: "trust the LLM" pushes ALL enforcement to the embedder, so out of the box there is no per-tool policy, no rate/budget clamps, no egress rules. No provenance or audit trail of its own — history lives in LangGraph checkpoints and observability is delegated to LangSmith cloud. Memory backends are pluggable but carry no scoping/ACL concept (no departments, no per-scope reads). HITL is opt-in configuration, not a default posture. No resource governance (token budgets, wall-clock limits) documented in the README at all.
