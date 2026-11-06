# Onboarding plan (Milestone 5, desktop first)

Status: proposal for the user to decide. Nothing here is a decision yet. Written 2026-11-05 from the code as it stands.

## What exists today

- `lh-config` SCHEMA is the one list of settings. `ConfigFile` reads and writes `config.toml`; `normalize` checks values.
- `GET/PUT /v1/settings` (`crates/lh-core/src/settings_api.rs`) lists and changes settings, all or nothing. A paired browser may not change `provider`, `permissions`, `web`, `media`, `spending` or `memory.trust`. Team keys are mirrored into the store.
- `GET/PUT /v1/provider` and `POST /v1/provider/test` (`provider_api.rs`) save an endpoint and key and test it.
- `GET /v1/reports/preview` shows what a crash report holds (nothing is sent yet).
- `lh setup` (`crates/lh-tui/src/setup.rs`) is the step-by-step terminal form with a `Prompter` trait and scripted tests.
- The GUI has no first-run flow. `pages_ui/mod.rs::after_get` opens Settings on the Models page when `ProviderInfo::needs_first_run()` (status NotSet). `prefs.rs` holds the look (`theme`, `mode`, `glass_colours`, `glass_opacity`, `name`). `wheel.rs` is the colour wheel maths (hex, HSV, disc image).

## (a) Backend

### First-run state

One small file beside the settings file: `onboarding.json` in the config folder, same place as `crash-pending.json`.

```
{"version":1, "done":false, "step":"engine", "profile":null, "asked":{"web":"later","media":"later","crash":"later"}}
```

- `done` is true once screen 3 is finished or skipped. `step` lets a closed window resume. `asked` records each just-in-time question as `answered`, `later` (snoozed) or absent.
- A file, not a SCHEMA key, because it is state, not a behaviour choice, and `settings` should not list it.
- New module `crates/lh-core/src/onboarding_api.rs`:
  - `GET /v1/onboarding` returns `{done, step, profile, asked, needs_engine}`. `needs_engine` is true when `provider.url` is empty and no local endpoint answers.
  - `PUT /v1/onboarding` merges `{step?, done?, profile?, asked?}`.
  - `POST /v1/onboarding/reset` sets `done:false` (for "Run setup again" in Settings).
- Existing installs: if `config.toml` exists and `provider.url` is set, treat as `done:true` on first read. Nobody gets re-onboarded after an update.
- Local only. Like `provider`, refuse when `Via::Device`. A phone must not set the profile that governs sudo and spending.

### Profiles as presets

A profile is a named set of existing keys. It lives in `lh-config` as `PROFILES: &[Profile]` so the terminal, the GUI and tests share it. Applying writes through the same `normalize` and the same all-or-nothing rule as `PUT /v1/settings`.

| key | Careful | Balanced | Hands-off |
|---|---|---|---|
| `team.autonomy` | plan-first | plan-first | check-ins |
| `learning.mode` | propose | propose | silent |
| `spending.mode` | caps | estimate | estimate |
| `spending.approve_over_usd` | 0.05 | 0.25 | 1.00 |
| `spending.daily_cap_usd` | 2 | (empty) | (empty) |
| `permissions.mode` | ask-all | tiered | tiered |
| `permissions.sudo` | false | false | true |
| `permissions.show_diff` | always | always | untrusted |
| `tools.sandbox` | true | true | true |
| `tools.sandbox_network` | ask | ask | auto |
| `team.unavailable` | stop | stop | fallback |
| `memory.enabled` | true | true | true |

Notes on the table:
- `fire-and-forget` is never in a preset. It is the only autonomy value with no questions at all, so it stays a deliberate choice in Settings. Hands-off uses `check-ins`. (Open question 2.)
- `permissions.bypass` is false in every profile. Bypass is never offered by onboarding.
- Hands-off sudo is true only after a second confirm line on the screen: "Agents may ask to use sudo. It still asks each time." Sudo still asks like any command (decision log).
- `tools.sandbox_network = auto` needs `tools.sandbox_network_model`. Hands-off falls back to `ask` while that is empty, and says so.
- `learning.mode` is not live (needs restart). `PUT /v1/onboarding` returns the existing `restart_needed` list; the GUI shows "Applies after a restart" once, with a restart button if the app started the core.
- Custom: applies nothing. It opens the Settings pages (Learning, Tools, Spending) and marks the profile `custom`. A profile is a one-time starting point, not a lock: later edits never "break" it. Settings shows "Started from Balanced, 3 changes since" computed by comparing the current values to the preset.

Endpoints: `GET /v1/profiles` returns each preset with the values and a plain sentence per row ("Asks before spending more than 25 cents"). `POST /v1/profiles/{name}/apply` applies and records `profile` in `onboarding.json`. Each change is logged as `setting_changed` with source `onboarding`, so the activity page shows it.

### Endpoint auto-detect

`GET /v1/provider/detect` (local only). The core probes loopback only, with 400 ms timeouts, in parallel:

| what | probe | url it proposes |
|---|---|---|
| llama-swap / llama.cpp | `GET :9292/v1/models`, `:8080/v1/models` | `http://localhost:9292/v1` |
| Ollama | `GET :11434/api/tags` | `http://localhost:11434/v1` |
| LM Studio | `GET :1234/v1/models` | `http://localhost:1234/v1` |
| LastEngine (pending) | `GET :<port>/v1/models` plus a marker header | its own url |

Returns `[{kind, url, name, models:[…first 5], count, key_needed:false}]`. Never scans other hosts or the network. Reuses `check_url` and the `looks_like_ollama` hint. A failed probe is silent; nothing found is a normal answer, not an error. Cloud: the GUI offers OpenRouter, OpenAI and "other" with a paste-key field; the key goes to `PUT /v1/provider` and into the secret store as `secret:NAME`, never into `config.toml`.

### Just-in-time asks

Mechanism: the core decides when, the GUI draws. A feature that needs an unanswered choice returns a typed event instead of failing.

- Core event `need_choice` on `/v1/events`: `{id:"web"|"media"|"crash", why, options:[{value, label, note}], safe_default}`. Raised once per question, only after `onboarding.done` and only when `asked[id]` is absent.
- Triggers:
  - `web`: the model first calls `web_search` and `web.searxng` is empty and no provider is set. Options follow the decision log: none, paid API with key, subscription search, existing SearXNG, app-managed SearXNG (shows its RAM cost), free provider (TinyFish). Safe default: none.
  - `media`: first `generate_image` request or first image model seen in `/v1/models`. Options: off, on with this model. Safe default: off (matches `tools.generate_image=false`).
  - `crash`: the first crash after a restart (a `crash-pending.json` exists). Shows the report preview from `/v1/reports/preview`, then off / ask each time / send automatically. Safe default: off.
- Answer: `POST /v1/choices/{id}` `{value}` writes the matching settings and sets `asked[id]="answered"`. "Not now" sets `later`, and the question can come back at most once per week, then stops asking and the feature stays off.
- The tool call that raised it returns "web search is not set up; the user was asked" so the model does not loop.
- The system prompt does not change. The question lives in the app, not in model text (budget rule).

### Backend tests

- Profile table: every key exists, every value passes `normalize`, presets differ only in the intended keys, no preset sets `permissions.bypass` or `fire-and-forget`.
- Apply is all-or-nothing and logs one `setting_changed` per key; `custom` writes nothing.
- First-run state: missing file with empty config gives `done:false`; missing file with a configured provider gives `done:true`; corrupt file is reported, not replaced.
- Detect: fake servers on random ports (the probe ports are a parameter); timeout does not hang; non-OpenAI answer is ignored; nothing is probed off loopback.
- Just-in-time: event raised once, not again after answer, snooze window honoured, paired device cannot answer.
- Device refusal for every new route.
- `lh setup` keeps working and uses the same `PROFILES` (add a "pick a profile" question first; keep the per-key questions as "Custom").

## (b) GUI

Follow the "Quiet desk" and liquid glass direction (`docs/liquid-glass-plan.md`). I do not invent branding: use the brand mark and motion kit already in `brand.rs` and `motion.rs`, and the active theme's tokens from `theme.rs`/`style.rs`.

### Shell

- A full-window overlay on first launch, over the glass background, no sidebar, no dock. One centred card, 520 px wide, one idea per screen, large calm headline, one primary button at the bottom.
- Progress: three small dots under the card, not a bar. Back is a text button on the left. Skip is never offered on screen 1 (nothing works without an engine) and is a quiet link on 2 and 3.
- State: a new `Page::Onboarding` in `layout.rs`, with `OnboardState {step, detect: Vec<Found>, test: TestResult, profile, theme_preview}` in a new `pages_ui/onboarding.rs` and pure logic in `pages_logic/onboarding.rs` (testable without a window, like `models_keys.rs`).
- Start: in `App::new`, `Cmd::Refresh` also fetches `/v1/onboarding`. If `done:false`, open the overlay. This replaces the current jump to Settings > Models. If the core is unreachable, show the existing "core is not running" state first (the app already starts it with `start_core_if_missing`).
- Motion: card fades and rises 8 px on step change (the existing `motion.rs` curves), instant when `reduce_motion()` is true.

### Screen 1: Connect an engine

- Headline: "Where do your models live?"
- On open it calls `/v1/provider/detect` and shows a list of found engines as selectable rows: icon, name ("llama-swap"), address, "6 models". One row is pre-selected if exactly one is found. A spinner row "Looking on this computer" for at most 1 s.
- Below the list, two quiet rows: "Use a cloud service" (opens provider chips OpenRouter / OpenAI / Other, an address field when Other, a key field masked, with a paste button) and "Set up a local engine for me" (disabled, labelled "Coming soon", until the engine slot exists; see c).
- Primary button "Connect": runs `/v1/provider/test`, shows "Reached it. 6 models." with the model sample (reuse `TestResult::model_sample`), then saves with `save_body`. On failure the message is the test's plain sentence and the card stays put.
- Nothing found and no cloud key: the card says so in one line and puts "Use a cloud service" and the future engine option forward. It never dead-ends.
- Privacy line under a cloud choice: "Memory marked private never leaves this computer." (matches the sensitive-data rule).

### Screen 2: How careful?

- Headline: "How much should it ask you?"
- Four large choice tiles in a 2x2 grid (one column below 560 px): Careful, Balanced (marked "Recommended", pre-selected), Hands-off, Custom. Each tile has a one-line promise and three short facts taken from `GET /v1/profiles` ("Asks before spending more than 25 cents", "Suggests, never changes rules by itself", "Sudo is off").
- A "See every setting this changes" disclosure lists the keys with their human names from `setting_names.rs`. This keeps the literal behaviour inspectable.
- Hands-off selected: an extra line with a checkbox "Let agents ask to use sudo" (default on for this profile, user can untick). Custom: button reads "Choose in Settings".
- Primary "Continue" applies the profile.

### Screen 3: Colour

- Headline: "Pick your colour"
- Left: swatch row of the theme families from `ThemeSet::families()` plus the colour wheel from `wheel.rs` (reused, with its brightness slider). Right: a live preview card showing a mini window (sidebar, a chat bubble, a button, a code chip) drawn with the real `Pal`, so the choice is the real look.
- Light / Dark / Follow the system switch under the preview (uses `appearance::ModeChoice`; Follow is selected).
- Changes apply live through `refresh_pal` and persist to `Prefs` (`theme`, `mode`, `glass_colours`). "Done" sets `onboarding.done` and fades into an empty new chat with the greeting from `greeting.rs`.
- Optional name field ("What should I call you?") only if the system name is empty. Otherwise skip, since `Prefs.name` already falls back to the system name.

### Just-in-time card

A small glass sheet that slides up from the composer (not a modal over the whole app), title = the question, one sentence of why, options as a vertical list with the safe default highlighted and focused, a "Not now" link. The crash question shows the report preview in a scrollable monospace box before the choice buttons.

### Arabic and right-to-left

- All text goes through the existing bidi helpers (`bidi.rs`, `rtl.rs`). The card mirrors: Back on the right, Continue on the left, progress dots reversed, tile grid fills right to left, icons that point (arrows, chevrons) flip.
- Addresses, API keys, model names and port numbers are always left-to-right runs inside Arabic text and in their own fields; the key field is LTR even in an RTL window.
- Number format follows the existing helpers; dollar amounts keep Western digits with the currency written the same way as the rest of the app.
- Language: the app's copy is English today. Plan the strings as one table (`onboarding_text.rs`) so an Arabic set can drop in. Arabic copy is written by the user or reviewed by them, not generated and shipped unreviewed. (Open question 5.)
- Test the RTL layout in unit tests on the layout logic (order of controls), not by driving the window.

### Keyboard and accessibility

- Tab order follows reading order and mirrors in RTL. Enter = primary, Esc = Back (never closes the flow), arrow keys move inside tile groups and engine lists, Space selects.
- Every tile and row has an accessible name and role (radio group, with checked state). The screen-reader tree in the app already exists; add the card as a landmark with the headline as its label.
- Step change announces the new headline. Errors from the connection test are announced and kept visible, not toast-only.
- Focus is visible with the theme's accent ring. Hit targets at least 40 px. Colour is never the only signal (selected tiles also get a check mark).
- Contrast: the colour picker rejects accent choices that fail 3:1 against the surface and shows a one-line note.
- `reduce_motion()` turns off the step transition and any preview animation.

## (c) Engine slot (pending)

Not built. LastEngine and hf-dl do not exist in this repo. The app needs a narrow interface so screen 1 can grow without a redesign. Proposed as a Rust trait in a new thin crate `lh-engine-api`, with a stub that returns `Unavailable` so the UI can be built and tested now.

```
trait EngineManager {
    fn scan_hardware(&self) -> Hardware;          // ram_gb, vram_gb, gpu name/vendor, cpu cores, free disk_gb, os
    fn recommend(&self, hw: &Hardware) -> Vec<Recommendation>;   // model id, size_gb, quant, why, fits: Fits|Tight|NoFit
    fn download(&self, model: &str) -> DownloadId;
    fn progress(&self, id: DownloadId) -> Progress;   // bytes_done, bytes_total, rate, state: Queued|Running|Paused|Done|Failed(reason)
    fn pause(&self, id) / resume(&self, id) / cancel(&self, id);
    fn start(&self, model: &str) -> Result<Endpoint>; // returns a url to feed into provider save
    fn stop(&self);
    fn status(&self) -> EngineStatus;                 // Stopped|Starting|Running{model}|Failed(reason)
}
```

- Exposed by the core as `/v1/engine/hardware`, `/v1/engine/recommend`, `/v1/engine/downloads` (POST, GET, DELETE) with progress on the existing SSE stream, and `/v1/engine/start|stop|status`. Local only.
- Screen 1 gains a "Set up a local engine for me" path: hardware card ("RTX 3050, 4 GB, 24 GB RAM"), 1 to 3 recommended models with plain fit labels, a download with progress and pause, then Start, then it saves the endpoint exactly like a detected one.
- Rules to honour: disk space check before download, resume after restart, checksum shown, no download without a click, nothing leaves the machine except the model download itself. The hardware scan stays on this computer.
- Until it exists, the entry is shown disabled with "Coming soon". Needs from the user or the engine work: licence and source of the recommended models, where files live (`~/AI` today), and whether the engine ships inside the app or as a separate download.

## (d) Phases

| # | Work | Effort | Risk |
|---|---|---|---|
| 1 | `PROFILES` in `lh-config`, tests, `GET /v1/profiles`, apply endpoint, activity log source | 1 day | Low. Preset values need the user's approval first. |
| 2 | `onboarding.json` state and `/v1/onboarding` routes, existing-install rule, device refusal | 0.5 day | Low |
| 3 | `/v1/provider/detect` with injectable ports and tests | 1 day | Low. Some servers answer slowly on cold start; use short timeouts and a manual "Look again". |
| 4 | `lh setup` uses profiles; Settings shows "Started from X, N changes" | 0.5 day | Low |
| 5 | GUI shell, screen 1 (detect, cloud, test, save), pure logic tests | 2 days | Medium. Key entry and masking in GPUI inputs; secret handling must match the Models page. |
| 6 | GUI screens 2 and 3 with live preview, theme and wheel reuse | 2 days | Medium. Live preview must use the real `Pal`; avoid a second render path. |
| 7 | Just-in-time: `need_choice` event, `/v1/choices`, web search and media asks, sheet component | 2 days | Medium. Must not loop the model or grow the system prompt. |
| 8 | Crash report ask with preview | 0.5 day | Low now; real sending is a separate unbuilt feature. |
| 9 | RTL and accessibility pass, keyboard walk, copy table | 1 day | Medium. Arabic copy needs the user. |
| 10 | Engine slot UI against the stub; real engine when LastEngine lands | 1 day plus engine work | High. Depends on a project that does not exist yet. |

About 11 working days before the engine. Phases 1 to 4 are backend and can land first with no GUI. Drive-by checks: run the app tests only (no window driving, per the mouse rule). Use `LFM3-1.7B-balanced` for the just-in-time web search run. Report the system prompt size in the milestone report; this plan adds zero bytes to it.

## (e) Open questions, one at a time

Ask these in order. Each has a recommendation worded so "yes" accepts it.

1. Should the three profiles hold the values in the table above, with Balanced as the pre-selected default? Recommended: yes.
2. Should `fire-and-forget` stay out of every preset, so Hands-off means `check-ins` and the full no-questions mode is only a deliberate Settings choice? Recommended: yes.
3. Should existing installs that already have a provider set skip onboarding, with "Run setup again" under Settings > General? Recommended: yes.
4. Should auto-detect probe only these loopback ports (9292, 8080, 11434, 1234) and nothing else, with a manual address field as the fallback? Recommended: yes.
5. Should the first-run copy ship in English now, with Arabic strings added after you review them? Recommended: yes.
6. Should the sudo choice sit only inside Hands-off (with a visible checkbox) and not appear in Careful or Balanced? Recommended: yes.
7. Should a snoozed just-in-time question come back at most once a week and then stop asking, leaving the feature off? Recommended: yes.
8. Should the live preview on screen 3 be a mini window drawn with the real theme, rather than applying the colour to the whole onboarding card? Recommended: yes.
9. Should "Set up a local engine for me" appear now as a disabled "Coming soon" row, so the layout does not change when LastEngine lands? Recommended: yes.
10. Should onboarding be refused from a paired phone or browser, as provider and permissions settings already are? Recommended: yes.
