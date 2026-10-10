# ViMax — Feature Dossier
Registry: #111 | Category: 07 | Verified: 12,084 stars, MIT, last push 2026-07-29

## Feature inventory
- **Idea2Video**: short concept → structured stories, characters, scripts, storyboards, shots, finished video end-to-end [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- **Script2Video**: explicit screenplay → controllable multi-scene multi-shot video "preserving its creative intent" [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- **Novel2Video**: long-form fiction → episodic visual narratives with narrative compression, character tracking, scene planning [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- **AutoCameo**: person/pet from a reference photo inserted into generated stories with consistent appearance throughout [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- **Agent Loop + TUI**: interactive planning/revision, rendering control, session resume (`vimax tui new/resume <session_id>`), context compaction; direct pipeline entrypoints preserved [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- **Web UI (v1.2.0)**: named projects, agent conversations with slash commands, artifact + storyboard previews, render checkpoints/progress, file uploads, provider settings; binds `127.0.0.1` only (SSH port-forward documented for remote) [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- **Consistency pipeline**: coordinate references, first frames, camera continuity, and final assembly across hundreds of shots; consistency validation catches unusable generations [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- **Parallelized generation** of compatible shots/media assets to accelerate multi-shot production [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- Provider config splits three roles — chat LLM (OpenRouter-hosted Gemini shown, any OpenAI-compatible `base_url`+`api_key`), image generator (`tools.ImageGeneratorNanobananaGoogleAPI`, Google API key), video generator (`tools.VideoGeneratorVeoGoogleAPI`, Google API key); env-var fallbacks `VIMAX_LLM_API_KEY` / `VIMAX_IMAGE_API_KEY` / `VIMAX_VIDEO_API_KEY` [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- Cloud generator expansion: OpenRouter GPT Image 2 image generation + Seedance 2.0 Fast video generation added 2026-07-17; Google Omni video generator 2026-06-01; MiniMax chat models 2026-03-23 — every documented media backend is a hosted API [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]
- Render checkpoints/resume: persistent render status, Script2Video resume fixes, landscape image guards [VERIFIED-fetched:https://github.com/HKUDS/ViMax/blob/main/readme.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Idea2Video / Script2Video / Novel2Video narrative-planning pipelines | 🏛 | Media Production dept candidate | A whole business function (concept→finished video with scripts/storyboards/shots as intermediate artifacts). No existing LastHarness department owns video production — this is the strongest wave-2 case for a new dept blueprint. |
| Character-consistency mechanism (reference images → first frames → camera continuity → assembly) | 🧩 | §14.3-style artifact chain for media | The mechanism that makes multi-shot AI video coherent is a pure pattern: pinned reference artifacts per character/scene, reused across generation calls, validated between steps. Portable regardless of which generator executes pixels. |
| AutoCameo identity insertion from reference photo | 🧩 | same artifact-chain pattern | Identity-pinning pattern extends the consistency mechanism; useful only if a local identity-preserving generator exists. |
| Chat-LLM planning role via OpenAI-compatible endpoint | ⚙ | Intelligence §14.4 → llama-server | `base_url` override means the screenwriter/director LLM can be pointed at local llama-swap with zero code change — this step passes the execution litmus after re-pointing. |
| Image generators (Nano Banana / GPT Image 2 — Google/OpenRouter APIs) | ❌ | — | Execution litmus fails: every documented image class calls third-party cloud APIs with customer content; violates local-first inference + default-deny egress. Needs a local SD-class swap before any adoption. |
| Video generators (Veo / Seedance 2.0 Fast / Google Omni APIs) | ❌ | — | Same litmus failure, worse: raw narrative content leaves the boundary shot-by-shot. No local video model path documented anywhere in the README. |
| Agent Loop + TUI (plan/revise/resume, context compaction) | 🧩 | §7.1 compaction protocol addendum; §16.x session recovery | Independent confirmation that long creative sessions need explicit compaction + resumable sessions; matches LastHarness's open compaction-protocol gap. |
| Web UI project workspace bound to localhost | 🧩 | §16.10 Owner Surfaces | Artifact previews, storyboard progress, render checkpoints = exactly the owner-approval surface shape; kernel-owned console should render these states. |
| Parallelized shot generation | 🧩 | §16.5 ephemeral worker fan-out | Shots are embarrassingly parallel; maps to fan-out of short-lived workers with assembled final output. |
| Render checkpoints / persistent render status | 🧩 | §13.2 recovery substrate | Resumable long-running generation jobs; checkpoint granularity is the transferable idea. |

## Notable absences
No offline/local generation path at all — orchestration is local Python but every pixel/frame comes from cloud APIs, so the tool is unusable under default-deny egress as shipped; no audio/music/TTS pipeline documented despite "Audio and Video Binding" marketing bullets; no licensing guidance for generated likenesses (AutoCameo of real people); Linux/Windows only (no macOS support statement); benchmark directory exists in-repo but no methodology in README.
