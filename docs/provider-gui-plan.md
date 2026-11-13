# Provider GUI plan: add any provider, paste a key and go

Status: plan for the user to decide. Nothing here is built or decided. Written 2026-11-07 from the code as it stands.
Sources read: AGENTS.md, docs/digest.md, docs/provider-presets.md, docs/onboarding-plan.md, crates/lh-provider/src/presets.rs, crates/lh-core/src/registry.rs, crates/lh-core/src/registry_api.rs, crates/lh-core/src/provider_api.rs, crates/lh-gui/src/extra_ui/providers.rs, crates/lh-gui/src/pages/providers.rs, crates/lh-gui/src/pages_ui/models_keys.rs (partly), crates/lh-core/src/model_tools.rs, crates/lh-provider/src/client.rs, crates/lh-provider/src/anthropic.rs.

## 1. What exists today

- Two provider surfaces exist. The registry (many providers, `providers.json`) is in `crates/lh-gui/src/extra_ui/providers.rs:31` (`pg_providers`), called from `crates/lh-gui/src/panels.rs:970` on the Models category. The older single-provider form is `crates/lh-gui/src/pages_ui/models_keys.rs:24` (`models_keys_block`), which talks to `PUT /v1/provider` (`models_keys.rs:114`). Both are live; the plan needs one of them removed (question 3).
- The registry API is in `crates/lh-core/src/registry_api.rs:20-25`: `GET/POST /v1/providers`, `GET /v1/providers/proposals`, approve and reject for proposals, `DELETE /v1/providers/{id}`, `POST /v1/providers/{id}/default`. There is no route to edit an entry, test an entry by id, or refresh an entry's model list.
- Adding works from a preset or an address: `Registry::add` (`crates/lh-core/src/registry.rs:202`) with `resolve_new` (`registry.rs:98`). The key goes to the secret store under `provider-{id}` (`registry.rs:209-211`); the entry keeps only `secret:provider-{id}`. Responses use `Entry::public` (`registry.rs:40-42`), which omits the key and the reference. Good.
- The preset picker is a list of chips with a note line (`extra_ui/providers.rs:118-146`). The GUI may check for a key before sending (`extra_ui/providers.rs`, `add_body`; not re-read line by line in this pass). The backend check for a key on a cloud preset was not confirmed in `resolve_new` (`registry.rs:98-119`); treat it as missing until phase 2 proves otherwise.
- The connection test is `POST /v1/provider/test` (`crates/lh-core/src/provider_api.rs:24, 112-138`). It takes `url`, `key`, `api`. It does not take a registry id. Without a typed key it uses `saved_key()` (`provider_api.rs:141-143`), which reads only the legacy `provider.key`. So testing a second registry entry needs the key typed again. This is a real gap for "paste key and go" and for re-testing.
- Model lists: `Registry::set_models` exists (`registry.rs:261`), and it is only written from the model tool `providers` list (`crates/lh-core/src/model_tools.rs:85-95`), with a 5 s timeout. No HTTP route refreshes a single entry's models. The GUI does not show a model list for registry entries.
- Presets: 26 entries (the request said 27; the array has 26) in `crates/lh-provider/src/presets.rs:62-89`, not 27 as the request said. Served by `GET /v1/providers/presets` (`provider_api.rs:28-40`). All 26 are `supported: true`; the "unsupported" branch is never used.
- Anthropic path: `Provider::anthropic` (`crates/lh-provider/src/anthropic.rs`), with `anthropic_models` falling back to a short list only on `api.anthropic.com` (`anthropic.rs:30-37`). Other Anthropic-style hosts get an error, not a fallback. Model list for OpenAI-style hosts: `GET {base}/models` (`client.rs:104`).
- Key handling: `SecretStore` trait with `OsStore` (system keyring, service `lastharness`) and `MemoryStore` (`crates/lh-secrets/src/lib.rs:17-80`). Core opens it in `crates/lh-core/src/main.rs:222`. Keys never go to `config.toml` or `providers.json`.
- Device rule: every write route refuses a paired browser or phone with HTTP 403 (`registry_api.rs:45-50`, `provider_api.rs:58-62`). Keep that for every new route.
- What is missing for "paste key and go": (a) a catalogue view where a card opens a prefilled form, (b) a key-only path for a known preset, (c) a kind selector and model list in the GUI, (d) a per-entry test and per-entry model refresh, (e) edit of an existing entry (name, url, kind, key replace or clear, models), (f) a backend rule that a cloud preset needs a key.

## 2. Preset list

Existing presets (26), with the base URL and help link source:

| id | Base URL | Where the URL is verified | Help link |
|---|---|---|---|
| llama-swap | http://127.0.0.1:9292/v1 | project default (AGENTS.md, onboarding plan) | none |
| ollama | http://localhost:11434/v1 | fetched 2026-11-05 (provider-presets.md:9) | docs.ollama.com/openai, from memory |
| lmstudio | http://localhost:1234/v1 | fetched 2026-11-05 (provider-presets.md:9) | lmstudio.ai doc, from memory |
| openai | https://api.openai.com/v1 | Pi source only (provider-presets.md:8) | from memory |
| anthropic | https://api.anthropic.com | Pi source (provider-presets.md:3) | from memory |
| google | https://generativelanguage.googleapis.com/v1beta/openai | fetched 2026-11-05 | from memory |
| openrouter | https://openrouter.ai/api/v1 | Pi source | from memory |
| groq | https://api.groq.com/openai/v1 | Pi source | from memory |
| together | https://api.together.ai/v1 | fetched 2026-11-05 | from memory |
| fireworks | https://api.fireworks.ai/inference/v1 | Pi source | from memory |
| deepseek | https://api.deepseek.com | Pi source | from memory |
| mistral | https://api.mistral.ai/v1 | fetched 2026-11-05 | from memory |
| xai | https://api.x.ai/v1 | Pi source | from memory |
| cerebras | https://api.cerebras.ai/v1 | Pi source | from memory |
| moonshot, moonshot-cn | api.moonshot.ai/v1, api.moonshot.cn/v1 | Pi source | from memory |
| kimi-code | https://api.kimi.com/coding/v1 | Pi source | from memory |
| zai, zai-cn | api.z.ai/api/coding/paas/v4, open.bigmodel.cn/api/coding/paas/v4 | Pi source | from memory |
| minimax | https://api.minimax.io/anthropic (anthropic kind) | Pi source | https://www.minimax.io (generic, not a key page) |
| qwen-token-plan | token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1 | Pi source | from memory |
| nvidia | https://integrate.api.nvidia.com/v1 | Pi source | from memory |
| huggingface | https://router.huggingface.co/v1 | Pi source | from memory |
| vercel | https://ai-gateway.vercel.sh/v1 | fetched 2026-11-05 | vercel.com doc, from memory |
| baseten | https://inference.baseten.co/v1 | Pi source | from memory |
| xiaomi | https://api.xiaomimimo.com/v1 | Pi source | from memory |

Every `help_url` is from memory (provider-presets.md:11). Check all of them before release.

Popular gateways missing from the list:

- OpenRouter, Groq, Together, Fireworks, DeepSeek, Mistral, Ollama, LM Studio: already present (see above). Not missing.
- llama.cpp server: UNVERIFIED port and base URL. Needs a check on the llama.cpp server docs. The onboarding plan uses `:8080` (docs/onboarding-plan.md:68), but provider-presets.md:16 says that port is not confirmed.
- vLLM: UNVERIFIED. Needs a check on the vLLM docs. provider-presets.md:16 notes `localhost:8000` as unconfirmed.
- LLM7.io: UNVERIFIED: needs a check on the provider's docs. Not mentioned anywhere in the repo, so there is no source here.
- DeepInfra: UNVERIFIED: needs a check on the provider's docs. provider-presets.md:17 lists it with no address.
- Other names in provider-presets.md (SambaNova, Novita, Venice, NanoGPT, Kilo, ZenMux, OpenCode, Cloudflare, SiliconFlow, Scaleway, LiteLLM): UNVERIFIED, no address in the repo (provider-presets.md:17).
- Perplexity, Azure OpenAI, Bedrock, Vertex, GitHub Models, DashScope: left out, no confirmed chat base (provider-presets.md:14-15). Azure and Bedrock need a different settings shape.

Note: the Pi source cited in docs/provider-presets.md is not in this repo, so "Pi source" rows were not re-checked in this pass. Only the "fetched 2026-11-05" rows come with a fetch date.

Rule for adding any preset: no URL goes in until it is checked against the provider's own docs and the check date goes in provider-presets.md. Until then the entry is listed as UNVERIFIED in the plan, not in the code.

## 3. GUI design

Fields for a custom provider (one form, used for new and edit):
- Name (text, required)
- Base URL (text, must start with http:// or https://; warn when not HTTPS and not local)
- API key: masked box with a show/hide eye toggle. Shows "saved" with a Replace and a Remove button when a key exists. Never shows the saved key.
- API kind: segmented control, two options: "OpenAI-compatible" and "Anthropic". Ollama appears only through the local preset, not as a third choice in the custom form. (Registry stores `openai`, `anthropic`, `ollama`; custom maps to `openai` or `anthropic`.)
- Model list: chips from the last successful refresh, a "Refresh models" button (calls the new route), a text box to add a model name by hand (for servers that do not list models).
- Test button: calls the new per-entry test route. Shows the plain message from the server and the model count.
- Save and Cancel. Remove sits on the card, not in the form.

Preset picker: a grid of cards with the provider name and one short note line, no logos, no brand colours. Grouped as "On this computer" (local presets) then "Cloud". Choosing a card opens the same form prefilled from the preset: name and base URL filled and editable, kind set from the preset, the key field focused, and a "Get a key" text link (help_url, opened in the system browser). Unsupported presets are greyed (none today) and cannot be saved. Cloud presets with `key_required` cannot save with an empty key. A "Custom provider" card at the end opens the empty form.

The brand and visual choices are the user's (AGENTS.md). The layouts below are options, not decisions.

Layout option A: catalogue first (recommended).
```
Providers                                  [Add a provider]
  [ Default ]  My Box        http://localhost:9292/v1     key not needed     [Test] [Edit] [Remove]
  [          ] OpenRouter    https://openrouter.ai/api/v1 key saved          [Test] [Edit] [Remove]

Add a provider
  On this computer:  [llama-swap]  [Ollama]  [LM Studio]
  Cloud:             [OpenRouter]  [Groq]  [Together AI]  [OpenAI] ... [Custom provider]
                                  (cards wrap; scroll on phone width)

  -- after a card is chosen --
  OpenRouter. One key, many models.          [Get a key]
  Name            [OpenRouter        ]
  API key         [************] [eye]   (needed)
  > Advanced: address, kind, models, test
  [Add]  [Cancel]
```
Why: the user sees the choice first, then only one field is needed (the key). The advanced part stays shut until asked.

Layout option B: one form with a preset dropdown on top (the plan in docs/provider-presets.md:20-26).
```
Preset  [ OpenRouter v ]   Name [OpenRouter]   Address [https://openrouter.ai/api/v1]
Kind    (o) OpenAI-compatible  ( ) Anthropic
API key [********] [eye]        Models: gpt-x, llama-y  [Refresh]   [Test]
[Save]  [Cancel]
```
Why it is weaker: every field is on screen at once, so a first-time user sees more than they need. It also keeps the old single-provider flow shape.

Recommendation: option A. Cards answer "what do I use", and the key field is the only required input. Option B stays as the form inside A's "Advanced" section.

## 4. Backend changes

Existing and reusable:
- `GET /v1/providers`, `POST /v1/providers` (preset or url, key), `DELETE /v1/providers/{id}`, `POST /v1/providers/{id}/default`: `registry_api.rs:20-25`.
- `GET /v1/providers/presets`: `provider_api.rs:24-25`.
- `POST /v1/provider/test`: `provider_api.rs:112`. Takes a url and key, not an id.
- `Entry.models` and `Registry::set_models`: `registry.rs:261`.

Missing (new routes, all local-only through the same `no_device` check):
- `PATCH /v1/providers/{id}`: name, url, api, trust, key action (`keep` | `replace` with a new key | `remove`), and models. The key replacement writes the same secret name `provider-{id}`, so no new secret name is needed. The response uses `Entry::public`, never the key. Add `Registry::update` in `registry.rs`.
- `POST /v1/providers/{id}/test`: uses the entry's own key from the secret store (`Registry::key_of`, `registry.rs:170`), so the user does not retype it. Reuse the `test` logic from `provider_api.rs:112-138` by moving the message and provider-building code into a shared function. An optional `key` in the body overrides it, as today.
- `POST /v1/providers/{id}/models`: refresh the list live (same 5 s timeout as `model_tools.rs:90`), save with `set_models`, return the list.
- Fix `resolve_new` (`registry.rs:98`): if the preset has `key_required` and no key is supplied, return an error. Then the backend enforces what the GUI already checks.

Secret store rules (keep):
- Key only in `SecretStore` (keyring via `OsStore`). Never in `providers.json`, `config.toml`, logs, or responses. Log lines carry `"key": "saved"` or `"removed"`, as in `registry_api.rs:72`.
- Remove deletes the secret (`registry.rs:239-242`).
- Keys are trimmed before saving (`registry.rs:207`).

Tests for backend changes: unit tests in `registry.rs` (update, key replace and remove, cloud preset without key refused, per-entry key used by test) and integration tests like `crates/lh-core/tests/provider_api.rs` (the fake-server pattern used at lines 129-163).

## 5. Phases

Each phase is a finished, building, tested change, committed by the user's identity (AGENTS.md; no agent trailers). Builds and model loads need the `~/AI/.machine-busy` lock. Do not run `crates/lh-gui/tests/gui_drive.py`.

| # | Work | Files | Test | Subagent? |
|---|---|---|---|---|
| 1 | Backend: per-entry test and model refresh routes; shared test function | `crates/lh-core/src/registry_api.rs`, `crates/lh-core/src/registry.rs`, `crates/lh-core/src/provider_api.rs` | `cargo test -p lh-core` (new integration cases with fake server, like `provider_api.rs:163`) | Yes |
| 2 | Backend: `PATCH /v1/providers/{id}`, key keep/replace/remove, cloud-needs-key rule in `resolve_new` | `registry.rs`, `registry_api.rs` | `cargo test -p lh-core` (registry unit tests, extend `first_is_default_keys_go_to_the_secret_store_only`) | Yes, after phase 1 (same files) |
| 3 | Catalogue data: decide which unverified presets to add (only after the docs check); add `help_url` checks to docs | `crates/lh-provider/src/presets.rs`, `docs/provider-presets.md` | `cargo test -p lh-provider` (existing `ids_are_unique_and_urls_are_well_formed`, `cloud_is_https_and_needs_a_key_local_is_local`) | Yes, but needs the user's answer to question 1 first |
| 4 | GUI: preset cards and prefilled add form (layout A) | `crates/lh-gui/src/extra_ui/providers.rs`, new pure logic in `crates/lh-gui/src/pages_logic/` | `cargo test -p lh-gui` on pure logic only (like `pages_logic/models_keys.rs`): card grouping, prefill, key-required gate | Yes |
| 5 | GUI: custom form (kind, masked key with eye, model chips, refresh, test), edit of an entry; remove the old single-provider block from Models if question 3 says yes | `crates/lh-gui/src/extra_ui/providers.rs`, `crates/lh-gui/src/pages_ui/models_keys.rs` | `cargo test -p lh-gui` pure logic; `cargo build -p lh-gui` | Yes, one subagent after phases 1 and 2 |
| 6 | Docs in step: provider-presets.md (API list, flows), spec `docs/spec/lastharness.md` provider section, decision log `.grill/lastharness-reboot.md`, digest if needed; milestone note if the user asks | docs files only | none beyond review by the main session | No, main session |

Phases 1 and 2 can each be done by one subagent. Phase 3 can be done by one subagent once the URLs are checked. Phases 4 and 5 are GUI work; a subagent can do each, and the main session checks the diff. Phase 6 is the main session's job, since it records decisions.

## 6. Open questions

Ask one at a time, in this order. Each recommended answer is worded so that "yes" accepts it.

1. Should LLM7.io, DeepInfra and llama.cpp server be added as presets only after their own docs are checked and their base URLs written into docs/provider-presets.md with the check date? Recommended: yes.
2. Should the current 26 presets stay as they are, with every `help_url` checked against the provider's own key page before the first public release? Recommended: yes.
3. Should the Models page show only the registry (cards and forms in `extra_ui/providers.rs`), and the old single-provider block in `pages_ui/models_keys.rs` be removed once the registry covers it? Recommended: yes.
4. Should each provider keep its own key in the secret store under `provider-{id}`, as now, with no change to the storage scheme? Recommended: yes.
5. Should the Test button for a saved provider use its saved key without asking the user to type it again? Recommended: yes.
6. Should the Models page use layout A (preset cards first, then one key field, with the rest under "Advanced")? Recommended: yes.
7. Should the backend refuse a cloud preset with no key, so the GUI check is not the only guard? Recommended: yes.
8. Should the model list refresh automatically after a save or edit, and also have a manual Refresh button? Recommended: yes.
