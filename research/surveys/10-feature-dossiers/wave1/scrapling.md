# Scrapling — Feature Dossier
Registry: #100 | Category: 03 | Verified: 75,868 stars, BSD-3-Clause, last push 2026-08-22 (GitHub API 2026-08-23)

Adaptive anti-bot scraping framework: single-request fetching through full concurrent crawls, with a parser that self-heals across site redesigns. Pure Python library (3.10+), runs entirely on the local machine — execution stays inside LastHarness's boundary when invoked by ephemeral workers.

## Feature inventory
- Adaptive element tracking: save element fingerprints at scrape time (`auto_save=True`), re-find them after redesigns with similarity algorithms (`adaptive=True`) — selectors survive website changes [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Three fetcher tiers: `Fetcher` (HTTP with browser TLS-fingerprint impersonation + HTTP/3), `DynamicFetcher` (Playwright Chromium/Chrome automation), `StealthyFetcher` (fingerprint spoofing; automates past Cloudflare Turnstile/Interstitial) [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Session classes (`FetcherSession`/`StealthySession`/`DynamicSession` + async twins) persisting cookies/state across requests; async browser tab pool with stats [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- `ProxyRotator` with cyclic/custom strategies across all session types plus per-request overrides [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Domain & ad blocking: block requests to specific domains+subdomains; built-in blocklist of ~3,500 ad/tracker domains in browser fetchers [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- DNS-over-HTTPS leak prevention routing DNS through Cloudflare DoH while proxying [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- `capture_xhr` background API capture: pass a URL pattern, collect matching XHR/fetch responses as Response objects without reverse-engineering site APIs [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Scrapy-like Spider framework: configurable concurrency, per-domain throttling, multi-session routing by session ID, blocked-request detection + retry, item streaming, built-in JSON/JSONL/CSV/XML exporters [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- AutoThrottle: per-domain delay auto-tuned from observed response behavior, doubles or honors `Retry-After` under blocking/rate-limiting, speeds back up after [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Pause/resume crawls: checkpoint persistence via `crawldir`; Ctrl+C pauses gracefully, restart resumes [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Optional `robots_txt_obey` compliance honoring Disallow/Crawl-delay/Request-rate with per-domain caching [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Ready-made spider templates: CrawlSpider, SitemapSpider, XMLFeedSpider/CSVFeedSpider, ShopifySpider (full store catalog via its JSON API); standalone LinkExtractor with allow/deny patterns and domain filters [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Built-in MCP server for AI-driven extraction (extract-before-LLM to cut tokens), can hold browser sessions open across calls, screenshot pages, drive remote browsers over CDP [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- Agent Skill directory teaching coding agents the current Scrapling API [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]
- CLI + interactive shell: `scrapling extract <url> out.md|.txt|.html` (Markdown/text/HTML export) without writing code; dev-mode disk cache replays responses for parser iteration [VERIFIED-fetched:https://raw.githubusercontent.com/D4Vinci/Scrapling/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Library adoption as Search/Data dept fetcher+parser engine | ⚙ Feature adoption | Information dept scraping worker tooling | Local execution, BSD-3-Clause, invoked per-dispatch by ephemeral workers — invariant-clean replacement for hand-rolled fetchers |
| Adaptive/self-healing element relocation | ⚙ Feature adoption | Scheduled scraping jobs durability | Extraction jobs keep working across site redesigns — directly cuts maintenance load LastHarness's Automation jobs would otherwise generate |
| AutoThrottle + robots_txt_obey | ⚙ Feature adoption | Egress gateway crawling policy | Politeness enforced mechanically (per-domain backoff, Retry-After) matches deny-by-default egress discipline and keeps the owner's IP welcome |
| Domain/ad blocklist + DoH leak prevention | ⚙ Feature adoption | Default-deny egress gateway defaults | Ready-made deny-lists and DNS hygiene fold straight into gateway config |
| `capture_xhr` declarative network capture | 🧩 Mechanism only | Data-extraction contracts § | Pattern: declare the data, capture what matches — auditable alternative to reverse-engineering private APIs |
| Pause/resume checkpoints | ⚙ Feature adoption | Long-running Automation job state | Resumable crawl state fits LastHarness's job supervision without any standing process |
| ProxyRotator abstraction | 🧩 Mechanism only | Egress egress-exception interface | Rotation interface is reusable if an allow-listed egress exception ever needs it |
| StealthyFetcher Cloudflare/Turnstile bypass | ❌ Conflict-skip | Deferred like §16.4 risk-concentrated scope | Anti-bot circumvention concentrates ToS/legal risk; V1 excludes, revisit only behind explicit owner opt-in policy |
| Bundled sponsor proxy services (residential proxies etc.) | ❌ Conflict-skip | — | Third-party paid services outside the self-hosted boundary |
| MCP server holding browsers open across calls | ❌ Conflict-skip as-is / 🧩 pattern | Kernel-mediated tool exposure instead | Standing browser session violates ephemeral-worker rule; expose extraction as per-dispatch kernel tools, not a resident MCP process |

## Notable absences
No scheduler/queue — spiders run when called, so cadence must come from LastHarness's Automation dept. No persistent datastore beyond exporter files. No search-engine aggregation or metasearch. No document-format conversion beyond CLI md/txt/html dumps of fetched pages (deep doc→markdown belongs to #104-class tools). Browser tiers require Playwright browsers installed (`scrapling install`) — installer must provision them inside the Linux environment. No built-in auth/session vault for logged-in scraping targets.
