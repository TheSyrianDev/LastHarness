# ScrapeGraphAI (Scrapegraph-ai) — Feature Dossier
Registry: #110 | Category: 03 | Verified: 29,852 stars, MIT, last push 2026-08-23

## Feature inventory
- `SmartScraperGraph`: single-page extraction from a prompt + source URL returning a structured dict (company description, founders, social links example); the canonical "just say which information you want" entrypoint [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- Named pipeline-graph family: `SearchGraph` (top-n search results, multi-page), `SpeechGraph` (scrape → audio file), `ScriptCreatorGraph` (scrape → generates a reusable Python scraping script), plus `SmartScraperMultiGraph`/`ScriptCreatorMultiGraph` multi-page variants [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- Direct-graph (digraph) pipeline construction: LLM + graph logic assemble the scraping pipeline instead of hand-coded selectors [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- LLM-agnostic config: OpenAI, Groq, Azure, Gemini, MiniMax via API **or fully local via Ollama** (`ollama/llama3.2` shown as first-class example with `format: json`) [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- Playwright-based fetching: `playwright install` required for website content, headless mode configurable [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- Local document ingestion: same graph pipelines run against XML, HTML, JSON, Markdown files — not just URLs [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- Parallel LLM calls in the *Multi graph versions* ("allows to make calls of the LLM in parallel") [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- Deliberate OSS-vs-cloud split: this library is self-hosted ("your own infrastructure", "on-prem/self-hosted data", BYO keys/browser/proxies) while the managed API (separate `scrapegraph-py`/`scrapegraph-js` SDKs, paid credits) adds stealth rendering, anti-bot, Crawl and scheduled Monitor jobs [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- MCP server published for agent-tool integration [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]
- Anonymous usage telemetry on by default; opt-out via `SCRAPEGRAPHAI_TELEMETRY_ENABLED=false` [VERIFIED-fetched:https://raw.githubusercontent.com/ScrapeGraphAI/Scrapegraph-ai/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| `SmartScraperGraph` prompt→structured-dict extraction | 🧩 | §14.3 extraction wrapper contract | Prompt-conditioned schema'd extraction over one fetched page is exactly the Research-dept extract step; with Ollama it runs entirely in-boundary. Contract shape worth mirroring in the SearXNG/scrape wrapper. |
| Digraph pipeline construction (LLM-composed steps) | 🧩 | §14.3 pipeline composition | Graph-of-steps beats hardcoded selector chains for maintainability; pattern portable to any local LLM. |
| `ScriptCreatorGraph` scrape→Python-script synthesis | ⚙ | Research §14.3 | Turns one-off extractions into reviewable, replayable artifacts — fits LastHarness's decision-log ethos: generated script becomes the auditable record of how data was obtained. |
| Ollama/local-model path with `format: json` | ⚙ | Intelligence §14.4 | Direct precedent for pointing scraping pipelines at llama-server/llama-swap instead of cloud LLMs; JSON-format constraint is the reliability hook. |
| Playwright headless fetching | 🧩 | §7.1 egress-gated fetch tier | Browser-class fetching belongs behind the default-deny egress gateway as an allow-listed fetch backend, not an ad-hoc network caller. |
| Local-document graphs (XML/HTML/JSON/MD) | 🧩 | §14.3 offline extraction | Same extraction contract against already-downloaded corpus — zero egress, useful for re-processing archived pages. |
| Multi-graph parallel LLM calls | 🧩 | §16.5 parallel fan-out | Precedent for chunked parallel extraction across many pages via ephemeral workers rather than one long-running job. |
| `SearchGraph` (top-n search-engine results) | ⚙ | §14.3 (behind SearXNG) | Search-result harvesting exists in-boundary via SearXNG; this graph defines the consumption side of it. |
| `SpeechGraph` (scrape → audio) | ⚙ | — | Niche output medium; no LastHarness consumer for spoken digests. Mechanism harmless, priority nil. |
| Managed cloud API + SDKs (credits, stealth, Monitor) | ❌ | — | All extraction executes on ScrapeGraphAI cloud; violates local-first + default-deny egress. Benchmark reference only. |
| MCP server | ⚙ | §4.4 kernel-mediated tool surface | Tool exposure is fine only if registered through the kernel's deny-by-default allow-list, not wired peer-to-peer into agents. |
| Default-on telemetry | ⚙ | Security §14.7 install checklist | Must be flipped off at deployment; default-on phoning-home from a data-extraction library is an egress-policy exception waiting to happen. |

## Notable absences
No proxy/anti-bot story in the OSS library (explicitly "your responsibility") — heavy-JS or bot-walled targets may fail in-boundary; no structured partial-failure response like the cloud API's per-item errors (OSS raises, cloud returns `failed_results`-style arrays); no scheduled/recurring jobs in OSS (Monitor is cloud-only — recurring scrapes need LastHarness's own scheduler); no diff/incremental re-crawl primitive; no data-retention or PII-scrubbing guarantees; upstream disclaimer limits use to "data exploration and research purposes".
