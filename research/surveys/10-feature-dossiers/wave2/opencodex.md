# OpenCodex — Feature Dossier
Registry: #62 | Category: 01 | Verified: 11,954★, MIT, last push 2026-08-24 (repo-verify.txt 2026-08-24)

Local proxy translating Codex's Responses API into 40+ other providers — usable from OpenAI Codex, Claude Code, Claude Desktop, and Grok Build — with a ChatGPT account-pool manager doing quota-aware routing. Recorded conflict: general translation proxy vs §16.1 no-proxy stance (Path B = user-supplied only); ToS-gray pools.

## Feature inventory
- Universal provider proxy: translates Codex's Responses API into whatever a target provider speaks — streaming, tool calls, reasoning tokens, images, both directions [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Multi-client compatibility surface: Codex, Claude Code, Claude Desktop, and Grok Build all run "any model you point it at" through the one proxy [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- ChatGPT account pool for Codex auth: add multiple accounts, refresh 5h/weekly/30d quotas in the dashboard; new sessions auto-route to lowest-usage healthy account (round-robin and fill-first policies also available) [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Thread affinity pinning: existing sessions stay on the account that started them, with documented rebind triggers (quota re-evaluation, failover, exclusion, affinity expiry, 401/403/429 recovery) [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Combos: one virtual model id with failover or weighted round-robin across providers [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Sub-agents on any model: routed models appear in Codex's sub-agent picker with v1/v2 surface control and fallback chains [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- OAuth login-once for xAI, Anthropic, Kimi; or forwarded `codex login`, pasted keys, `${ENV_VAR}` references; web-search & vision sidecars ride the user's ChatGPT login for non-OpenAI models [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Web dashboard on localhost:10100 — providers, OAuth status, model picks, account management, live request log with cache token counts [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Autostart modes: `ocx service` background service (launchd/systemd/Task-Scheduler managed, restart-on-crash) or `ocx codex-shim` on-demand startup when `codex` launches [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Network hygiene: binds `127.0.0.1` by default, no auth needed on loopback; non-loopback binding requires a bearer token on every request [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Bounded memory ownership: 36 categories of retained process state each with documented byte caps/budgets (default 256 MiB app-owned budget), eviction counters inspectable via admin API [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Clean exit / zero residue: `ocx stop` restores Codex's original configuration; health/readiness endpoints (`/healthz`, `/readyz`) with fail-closed semantics [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]
- Maintainer disclaimer: independent project, not affiliated with providers; Anthropic notably may suspend accounts routing through third-party proxies — UAYOR, account pooling explicitly not endorsed for circumventing provider limits [VERIFIED-fetched:https://raw.githubusercontent.com/lidge-jun/opencodex/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| General translation proxy (any provider ↔ Codex/Claude CLIs) | ❌ Violates no-proxy stance (§16.1) | — | §16.1's Path B is a narrow single user-supplied-endpoint exception, explicitly "not general multi-provider routing"; a standing translation layer is exactly what it excludes |
| Hosting Claude Code/Grok Build as client surfaces | ❌ Violates SDK lock (§16.2) | — | Second vendor substrates are what §16.2 removed with jcode; every worker is a Claude Agent SDK subagent |
| ChatGPT account pool w/ quota-aware auto-routing | ❌ Violates attribution/auditability (§12/§13 auth chain); ToS-gray | — | Which identity performed an action must be answerable from the decision log; rotating subscription accounts makes attribution meaningless and invites provider enforcement (the README itself warns of suspension) |
| Thread-affinity pinning + rebind triggers | ❌ Same violation family as pooling (§12/§13) | — | Affinity management is pool plumbing; no compliant form while pooled identities exist |
| Combos: virtual model id with failover/round-robin | 🧩 Pure architecture pattern | §16.1 serving stack (`llama-swap`) policy slot | Failover across *locally served* model aliases is legitimate resilience; llama-swap already owns hot-swap, alias-level fallback is a config extension there — never cross-provider credential failover |
| Sub-agent picker surface control + fallback chains | ❌ Violates SDK lock (§16.2) | — | Orchestrating foreign engines' subagent surfaces presumes those engines exist in LastHarness; they don't |
| Web-search/vision sidecars over ChatGPT login | ❌ Violates default-deny egress (§13.4.2) + inference-path rules (Appendix A Path A/C) | — | Rides consumer-subscription credentials to third-party services outside any sanctioned inference path |
| Dashboard with live request log + cache token counts | ⚙ Capability owner console owns | §16.6 Output 3 traces → console render | Presentation over request telemetry; same data the audit trail already captures |
| Background service autostart mode | ❌ Violates ephemeral dispatch (§16.5) | — | Restart-on-crash resident daemon is a standing process; the shim's on-demand start is closer but still has no sanctioned role to play |
| Loopback-default binding + mandatory bearer token beyond loopback | 🧩 Pure architecture pattern | §13.4.2 gateway hardening default | Deny-by-orientation: closed unless deliberately opened with explicit auth — mirrors LastHarness's deny-by-default posture |
| Bounded memory ownership (byte-budgeted stores, eviction) | 🧩 Pure architecture pattern | §16.5 worker-harness design note | Finite-cap caches with inspectable budgets is the right discipline for any long-lived local component; informs harness hygiene, no new mechanism |
| Clean exit restoring original config | 🧩 Pure architecture pattern | §14.5 versioned-registration hygiene | Snapshot-before-mutate/revert-on-remove belongs in how LastHarness installs/uninstalls skills and connectors |

## Compliant re-implementation sketch
- The honest core: most of opencodex has no LastHarness form. The unified Claude Agent SDK (§16.2) means there is no second CLI to smuggle models into, and §16.1 fixes inference as local-first with at most ONE user-supplied custom endpoint.
- That sanctioned slot already exists: Appendix A Step 1 Path B — a single owner-supplied endpoint behind a mandatory privacy warning. It stays singular; no multiplexer, no provider catalog, no per-model translation matrix.
- The "never stuck on one model" value lands legitimately as **alias-level failover inside the local serving stack** (§16.1 `llama-swap`): if the primary alias times out, serve the fallback local alias — same modality tier, zero external calls.
- Adopt loopback-default + explicit-token-for-anything-wider as the gateway's binding idiom (§13.4.2 hardening defaults).
- Adopt bounded-memory accounting as a worker/harness hygiene note: every cache gets a declared cap; eviction counters land in traces for the owner console.
- Adopt snapshot-and-restore on install/remove for skills and connector registration (§14.5), giving LastHarness the same zero-residue exit.
- Stay out: provider translation itself, multi-account pools (attribution + ToS), sidecars on consumer logins, background service mode, foreign client hosting.

## Notable absences
No audit log of which account/model answered which user task beyond the live dashboard ring; no per-user or per-task cost attribution (pooling actively prevents it). No egress allow-listing — once configured, every provider endpoint is equally reachable. The maintainer disclaimer confirms the central risk rather than mitigating it: Anthropic may suspend proxy-routed accounts. Documentation breadth (opencodex.me) was not fetched; README-level claims only.
