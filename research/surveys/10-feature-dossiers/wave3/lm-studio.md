# LM Studio (lms) — Feature Dossier
Registry: #57 | Category: 04 | Verified: 5,222 stars, MIT, pushed 2026-08-18; repo is the CLI only — the LM Studio app itself is closed-source (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: named Path B endpoint in Appendix A Step 1 — confirmed verbatim ("Connect my own AI server" (Path B) — subtext: "Already running Ollama, LM Studio, or something similar? Point LastHarness at it."), and §16.1's Path B proxy exception names "a self-hosted … LM Studio instance" as the user-supplied-endpoint case. Server-side features relevant to custom-endpoint support are inventoried below.

## Feature inventory
- **`lms` CLI ships with LM Studio ≥0.2.22**: `status`, `server start`/`server stop`, `ls --json`, `ps --json` (loaded models), `load <path> -y` (max GPU without confirmation), `unload [--all]`, `log stream`, `create` (SDK project scaffold); built on lmstudio.js [VERIFIED-fetched:https://raw.githubusercontent.com/lmstudio-ai/lms/main/README.md]
- **OpenAI-compatible server endpoints**: chat completions, completions (legacy), embeddings, list models, Responses API, structured output (JSON schema), tool use — the surface a custom endpoint consumer integrates against [VERIFIED-fetched:https://lmstudio.ai/docs/developer]
- **Anthropic-compatible Messages endpoint**: "Use Claude-style Messages API flows against your local LM Studio server" [VERIFIED-fetched:https://lmstudio.ai/docs/developer]
- **Headless daemon (`llmster`)**: LM Studio's core packaged as a GUI-independent daemon for servers/CI — `lms daemon up`, `lms get <model>`, `lms server start`; install script for Mac/Linux/Windows [VERIFIED-fetched:https://lmstudio.ai/docs/developer]
- **REST API beyond OpenAI shape**: stateful chats, streaming events, model load/download/unload/download-status over HTTP with Bearer-token auth; MCP via API; idle-TTL auto-evict of loaded models [VERIFIED-fetched:https://lmstudio.ai/docs/developer]
- SDKs: TypeScript (`@lmstudio/sdk`) and Python (`lmstudio`) clients [VERIFIED-fetched:https://lmstudio.ai/docs/developer]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| LM Studio as a Path B custom endpoint | 🧩 Mechanism only | Confirms Appendix A Step 1 Path B naming + §16.1 proxy exception scope | The recorded skip confirmed: LastHarness already supports exactly this — an owner-run LM Studio endpoint is a user-supplied Path B provider, warned per Step 1D and never general multi-provider routing. |
| OpenAI-compatible endpoints incl. tool use + structured output | 🧩 Mechanism only | §16.1 Path B consumer contract | These are precisely the capabilities LastHarness's worker roles need from any custom endpoint; their presence confirms Path B endpoints can serve full agent workloads. |
| Anthropic-compatible `/v1/messages`-style endpoint | 🧩 Mechanism only | Strengthens §16.1 Path B proxy exception | Same fact-class as llama-server's native Messages support that shrank the adapter: popular local servers increasingly speak Anthropic wire format natively, so the one-way translation module shrinks toward a no-op for this endpoint class. |
| `llmster` headless daemon + bearer-auth REST control plane | 🧩 Mechanism only | Pattern reference only | A viable way owners could run a Path B server inside their own boundary without a GUI; LastHarness neither bundles nor requires it — its own stack stays llama-server/llama-swap. |
| Idle-TTL auto-evict / explicit load-unload model management | 🧩 Mechanism only | Adjacent to llama-swap hot-swap §16.1 | Solves VRAM residency by TTL/explicit commands where llama-swap solves it by alias-triggered hot-swap; same one-model-resident constraint, different trigger. |
| Adopting LM Studio as LastHarness's bundled runtime | ❌ Conflict-skip | — | Closed-source desktop application duplicating §16.1's inspectable llama.cpp-based stack; MIT covers the CLI repo only, not the app. |

## Notable absences
No hardware-tier → pack mapping documented (model downloads are user-chosen, not tier-bundled like Appendix A Step 4). No alias layer: requests name concrete loaded models rather than stable logical aliases that survive model swaps — the specific gap llama-swap fills. No permission/approval or egress concepts (it is an inference server). License asymmetry worth restating: repo stars/license describe `lms` (MIT), while the product it controls is freeware-closed.
