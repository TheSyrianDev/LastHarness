# SearXNG — Feature Dossier
Registry: #90 | Category: 03 | Verified: 35,935 stars, AGPL-3.0, last push 2026-08-22 (GitHub API 2026-08-23)

Already ADOPTED in LastHarness §16.4 as the V1 self-hosted search choice (research/surveys/03-search-data.md). This dossier therefore skips re-adjudicating the metasearch concept and inventories the JSON-API/settings.yml wiring mechanics needed to embed it: what to flip, what stays off, and which of its own guardrails map onto LastHarness's invariants.

## Feature inventory
- Response-format deny-by-default allow-list: `search.formats` ships as `[html]` only; `json` (also csv/rss) must be explicitly added before `/search?q=...&format=json` answers — the JSON API is opt-in per format [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Loopback-by-default server posture: `bind_address: "127.0.0.1"`, port 8888, `public_instance: false`, `limiter: false` for private instances, `secret_key` override via `${SEARXNG_SECRET}` [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Engine failure suspension ladder: `ban_time_on_fail: 5s`, `max_ban_time_on_fail: 120s`, and per-cause `suspended_times` (AccessDenied 180s, CAPTCHA 3600s, 429/TooManyRequests 180s, Cloudflare captcha 1,296,000s, ReCAPTCHA 604,800s) [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Outgoing egress controls in one `outgoing:` block: `request_timeout: 3.0` default + optional `max_request_timeout`, HTTP/2 toggle, connection-pool sizing, SOCKS/HTTP `proxies.all`, `using_tor_proxy` with `extra_proxy_timeout`, multi-interface `source_ips`, custom CA `verify` path [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Plugin registry with explicit `active: true/false` toggles; active by default: calculator, hash, self_info, unit_converter, ahmia_filter, hostnames, time_zone, tracker_url_remover; off by default: infinite_scroll, oa_doi_rewrite, tor_check [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Hostnames plugin result rewriting: regex `replace` / `remove` / `low_priority` / `high_priority` lists over result domains, loadable from an external YAML file — sanitize or demote upstream results before they reach consumers [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Metrics gate: `enable_metrics: true` records stats; Prometheus-format exposition at `/metrics` disabled unless `open_metrics` password set [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Preferences lock list: arbitrary UI/preference keys (categories, language, safesearch, method, image_proxy…) can be locked against user change [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Per-engine configuration table (~200 engines): each with `disabled`/`inactive` flags, shortcut, timeout, weight, categories; offline indexer engines exist (elasticsearch, meilisearch, mongodb) alongside online ones [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]
- Optional Valkey URL for shared cache/state (`valkey.url: false` by default) and env-var overrides for nearly every key (`${SEARXNG_LIMITER}`, `${SEARXNG_BASE_URL}`, `${SEARXNG_IMAGE_PROXY}`…) [VERIFIED-fetched:https://raw.githubusercontent.com/searxng/searxng/master/searx/settings.yml]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| `formats: [html]` → add `json` explicitly | ⚙ Feature adoption | §16.4 wiring: installer writes minimal settings.yml enabling json only | The JSON API behind the Research dept WebSearch tool must be an explicit allow-list entry, never the shipped default — matches deny-by-default invariant |
| Loopback bind + public_instance:false | 🧩 Mechanism only | §16.4 deployment notes | Instance reachable only inside the LastHarness Linux boundary; kernel-mediated access, no external exposure |
| Engine ban/suspension ladder | ⚙ Feature adoption | Egress gateway health/backoff policy | Per-upstream escalating backoff (incl. long captcha suspensions) is exactly the state the egress gateway should track for search providers |
| `outgoing:` proxies/Tor/source_ips/timeouts | ⚙ Feature adoption | Default-deny egress gateway config schema | SearXNG's own egress knobs become the template for how the gateway grants search egress: pinned timeouts, optional proxy hop, source-IP pinning |
| tracker_url_remover + hostnames rewrite plugins | ⚙ Feature adoption | Search/Data dept result-sanitization step | Strip/demote tracking and junk domains before results enter worker context or any log |
| Password-gated /metrics exposition | 🧩 Mechanism only | Observability § | Optional scrape point; immutable decision log remains the system of record, metrics stay gated |
| Preferences lock list | 🧩 Mechanism only | Non-technical onboarding § | Pattern for freezing settings owners shouldn't wander into |
| Valkey dependency | ❌ Conflict-skip | — | Standing cache service violates ephemeral-worker/no-standing-process posture; single-owner scale needs none (precedent: SQLite+FTS5 ruling) |
| Offline indexer engines (elasticsearch/meilisearch/mongodb) | ❌ Conflict-skip | — | Each drags a standing service; LastHarness local corpus search stays on SQLite FTS5 |
| Public-instance limiter/bot-blocking | ❌ Conflict-skip | — | Meaningful only when exposed to strangers; LastHarness instance is localhost-only |

## Notable absences
No authentication layer on the API at all — anything that can reach the loopback port gets search; LastHarness's kernel must be the sole client (role allow-list at the gateway, not in SearXNG). No persistent result cache to disk, no per-role/per-tool query scoping, no audit trail of who asked what (LastHarness's decision log must capture queries kernel-side). Aggregation latency inherits upstream engine slowness; no SLA. Plugin code runs in-process without sandboxing.
