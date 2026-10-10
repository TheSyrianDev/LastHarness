# OpenCode — Feature Dossier
Registry: #42 | Category: 01 | Verified: 200,885 stars, MIT, pushed 2026-08-24 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: client/server split ≈ kernel/app surface; multi-model ≈ llama-swap aliases. README alone is feature-thin, so both skip reasons are confirmed from the project's official docs pages plus its raw README (dev branch).

## Feature inventory
- **Client/server architecture**: every `opencode` launch starts a TUI *and* a server — "Where the TUI is the client that talks to the server"; `opencode serve` runs a headless HTTP server exposing an OpenAPI 3.1 spec endpoint; "This architecture lets opencode support multiple clients"; IDE plugins drive the TUI through `/tui` endpoints [VERIFIED-fetched:https://opencode.ai/docs/server/]
- **Server auth is opt-in basic auth**: `OPENCODE_SERVER_PASSWORD` protects `serve`/`web`; default listen is `127.0.0.1:4096` [VERIFIED-fetched:https://opencode.ai/docs/server/]
- **Permission round-trip over the API**: `POST /session/:id/permissions/:permissionID` responds to permission requests with `{response, remember?}` [VERIFIED-fetched:https://opencode.ai/docs/server/]
- **75+ LLM providers** via AI SDK + models.dev, including first-class local entries for llama.cpp (`http://127.0.0.1:8080/v1`), LM Studio (`http://127.0.0.1:1234/v1`), and Ollama as custom OpenAI-compatible providers with per-model config [VERIFIED-fetched:https://opencode.ai/docs/providers/]
- **Model curation switches**: per-provider `blacklist`/`whitelist` options hide models from the `/models` picker [VERIFIED-fetched:https://opencode.ai/docs/providers/]
- **Built-in agents**: `build` (full access) vs `plan` ("Denies file edits by default", asks before bash) toggled with Tab, plus a `general` subagent for multistep tasks [VERIFIED-fetched:https://raw.githubusercontent.com/anomalyco/opencode/dev/README.md]
- **Desktop app (BETA)** for macOS/Windows/Linux alongside the TUI [VERIFIED-fetched:https://raw.githubusercontent.com/anomalyco/opencode/dev/README.md]
- **OpenCode Zen / OpenCode Go**: team-curated model lists behind an account with billing, "completely optional to use" [VERIFIED-fetched:https://opencode.ai/docs/providers/]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Client/server split (headless server, TUI/IDE/desktop as clients, OpenAPI spec) | 🧩 Mechanism only | Validates existing kernel/app surface separation | The recorded skip confirmed: LastHarness already splits execution authority (kernel + ephemeral workers, §16.5/§16.9) from owner-facing surfaces; opencode independently arrived at the same shape, headless-core-plus-thin-clients. |
| 75+ provider directory incl. localhost llama.cpp/LM Studio/Ollama entries | 🧩 Mechanism only | Validates existing llama-swap aliases in §16.1 | Multi-model is solved by pointing one agent at many endpoints; LastHarness's alias-routing over llama-swap already covers this without a provider catalog — aliases are the organizational version of opencode's provider config. |
| `blacklist`/`whitelist` model filters per provider | 🧩 Mechanism only | Adjacent to hardware-tiered pack curation | Pack selection (Appendix A Step 4) already narrows which models exist; picker-level filtering is the same lever applied later in the stack. |
| `plan` read-only agent denying edits by default | 🧩 Mechanism only | Validates §16.6 input 4 deny-by-default allow-lists | A shipped example that read-only scoping is expressed as an agent config slice, exactly where LastHarness puts enforcement (dispatch-time role config). |
| Permission request/response endpoint with `remember` | 🧩 Mechanism only | Owner approval round-trip §7.2 / §13.4 gate UX | Same approve-and-optionally-remember shape as the owner override flow; presentation-layer only over data dispatch already produces. |
| Desktop app product surface | ❌ Conflict-skip | — | General agent-workspace product surface LastHarness deliberately does not build; Personal Agent owns any chat/work UX. |
| OpenCode Zen/Go curated-model subscriptions | ❌ Conflict-skip | — | Cloud account + billing in the inference path; violates local-first default (cloud stays an explicit Path A-style opt-in at most). |

## Notable absences
No memory or knowledge-lifecycle story anywhere in README/server docs — sessions are listed/forked/summarized but nothing persists learned facts across tasks. Server auth is off unless the user sets a password (loopback-only by default mitigates). Session `share` endpoints publish transcripts to hosted share URLs — an egress surface LastHarness's gateway posture would never grant by default. No hardware profiling or pack-download flow: provider/model choice is fully manual. Nothing about egress policy, audit logging durability, or non-technical onboarding.
