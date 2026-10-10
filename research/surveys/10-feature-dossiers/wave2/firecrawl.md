# Firecrawl — Feature Dossier
Registry: #74 | Category: 03 | Verified: 171,568 stars, AGPL-3.0, last push 2026-08-24

SCOPE: §16.4 already excluded CLOUD Firecrawl for V1. This dossier's subject is the open-source SELF-HOSTED engine (AGPL-3.0 core in this repo); cloud-only features appear only as ❌ reference rows.

## Feature inventory
- `Scrape` endpoint (`POST /v2/scrape`): URL → markdown, HTML, screenshots, or structured JSON, with `--only-main-content` flag in CLI [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- `Search` endpoint (`POST /v2/search`): web search returning full page content per result (`url`, `title`, `markdown`) in one call [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- `Crawl` endpoint (`POST /v2/crawl`): whole-site crawl returning job ID, then `GET /v2/crawl/{id}` polling with `status`/`total`/`completed`/`creditsUsed`/`data[]`; SDKs poll automatically [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- `Map` endpoint (`POST /v2/map`): instant URL discovery of a site, with optional `search` param returning URLs ordered by relevance [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- `Batch Scrape`: thousands of URLs asynchronously as one job with per-doc `metadata.source_url` [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- `Interact`: scrape returns `metadata.scrape_id`, then `POST /v2/scrape/{id}/interact` drives the live page with AI prompts or code (click/scroll/write/wait/press); cloud variant returns a `liveViewUrl` [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- `Agent` endpoint (`POST /v2/agent`): autonomous no-URL data gathering with pydantic-schema structured output and source list; explicit "evolution of our `/extract` endpoint"; model choice `spark-1-mini` (60% cheaper) vs `spark-1-pro` [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- Self-host surface (this revision's Compose): Firecrawl API + workers, bundled Playwright with basic-fetch fallback, Redis, RabbitMQ, NuQ PostgreSQL queue (opt-in `NUQ_BACKEND=fdb` FoundationDB), Kubernetes manifests + Helm chart; only the API publishes to host, port 3002 [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/SELF_HOST.md]
- Self-host baseline posture: `USE_DB_AUTHENTICATION=false` means the default API is **unauthenticated**; Compose defines **no persistent volumes** for NuQ/Redis/RabbitMQ; queue admin UI off by default; AI-backed features ship with **no model provider** — connect OpenAI, an OpenAI-compatible endpoint, or Ollama [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/SELF_HOST.md]
- Anti-bot separation: scraping reliability machinery (Fire-engine-class engines, proxies) is NOT in the OSS bundle — self-host connects a separate engine "only when you need it" [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/SELF_HOST.md]
- Agent-intake surfaces: MCP server (`firecrawl-mcp`), CLI (`npx -y firecrawl-cli@latest init --all --browser`), build/workflow skills authored in-repo under `skills/` and CI-mirrored into a read-only catalog (`firecrawl/skills`), plus an agent-onboarding `SKILL.md` served over curl [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- Ten-language SDK family (Python/Node/Go/Java/Elixir/Rust/Ruby/.NET/PHP) with identical scrape/agent/crawl/search surface; SDKs and some UI components MIT inside an AGPL-3.0 repo [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- Politeness default: respects robots.txt directives by default; end users responsible for site policies [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]
- Cloud-only differentiators: 96%-of-web coverage claim, P95 3.4s, rotating proxies/orchestration — hosted-service features outside the OSS engine [VERIFIED-fetched:https://raw.githubusercontent.com/firecrawl/firecrawl/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| `Scrape` markdown/structured-JSON output contract | 🧩 | §14.3 extraction wrapper contract | Clean-markdown-per-URL with metadata is the same consumer shape the SearXNG wrapper must emit; keeping the contract identical makes backend swaps free. |
| `Search` results-with-full-content shape | 🧩 | §14.3 SearXNG wrapper contract | Search-plus-content-in-one-response is the merged step LastHarness composes locally from SearXNG + fetcher; this defines the expected output envelope. |
| `Crawl` job-ID + status-polling protocol | 🧩 | §14.3 crawl jobs via §16.5 dispatch | Long crawls decompose into ephemeral worker dispatches that persist progress; the poll/status fields (`completed`/`total`) are the right resume-checkpoint vocabulary. |
| `Map` + relevance-search URL discovery | 🧩 | §14.3 pre-crawl planning | Cheap link-space enumeration before spending fetches maps directly onto the Research dept's plan-then-extract flow. |
| `Batch Scrape` async fan-out | 🧩 | §16.5 parallel fan-out | Precedent for chunked many-URL extraction across parallel ephemeral workers rather than one resident crawler. |
| Self-hosted engine (Compose/K8s/Helm) | ⚙ | Research dept §14.3 infra candidate | Runs entirely in-boundary on the ONE Linux-env boundary; NuQ/Redis/RabbitMQ stack is heavy but containerized, no third-party runtime executing agent workloads. |
| Ollama / OpenAI-compatible hookup for AI-backed extraction | ⚙ | Intelligence §14.4 | Self-host explicitly accepts any OpenAI-compatible endpoint for its LLM features — i.e., llama-server drops in without code changes. |
| `Interact` AI-driven page interaction | 🧩 | §7.1 egress-gated fetch tier (advanced) | Browser-driving belongs behind the default-deny egress gateway as an allow-listed interactive-fetch backend; cloud `liveViewUrl` must never be wired. |
| `Agent` endpoint + spark-1-mini/pro models | ❌ | — | Inference executes on Firecrawl's cloud regardless of self-hosting the rest; violates local-first. Structured-output-with-sources remains a benchmark target only. |
| Hosted service (credits, stealth, 96% coverage) | ❌ | — | Third-party cloud execution of scraping workloads; §16.4 already excluded it for V1 — verdict stands. |
| MCP server / CLI / skills intake | ⚙ | §4.4 kernel-mediated tool surface + §14.5 intake lifecycle | Legitimate only as kernel-registered, deny-by-default tools and version-pinned skill intake — never peer-to-peer wiring from agents. |
| robots.txt-respect default | 🧩 | §7.1 egress policy hygiene | Default-polite fetching is the correct posture to codify in the egress gateway's fetch-tier rules. |
| Unauthenticated-by-default self-host API | ⚙ | Security §14.7 hardening checklist | Any in-boundary deployment must add auth/TLS/network policy first — the docs say so themselves; treat as mandatory hardening, not optional. |

## Notable absences
The legacy `/extract` endpoint is no longer documented in the README (named only as the Agent endpoint's predecessor), so self-host structured extraction currently routes through the cloud-model Agent path — a gap for fully-local structured extraction until the OSS extract pipeline is confirmed model-provider-complete. No anti-bot/proxy capability ships in the OSS engine (explicitly bring-your-own engine), so bot-walled targets may simply fail in-boundary. Compose baseline has no authentication, TLS, or persistent volumes — durability and access control are entirely the operator's design. No incremental/diff re-crawl primitive, no scheduled recurring crawls in OSS (cloud Monitor equivalent absent), no data-retention or PII-scrubbing guarantees, and credit metering exists only on the cloud side.
