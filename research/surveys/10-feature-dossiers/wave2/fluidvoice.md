# FluidVoice — Feature Dossier
Registry: #120 | Category: 04 | Verified: 10,856 stars, GPL-3.0, last push 2026-08-23

## Feature inventory
- macOS dictation app with on-device STT: global hotkey capture from anywhere, live-preview transcription overlay (notch-aware, configurable pill→large sizes), menu-bar integration [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Multi-engine STT menu with per-model language/size/hardware table: Nemotron Speech 3.5 (ultra-fast low-latency + multilingual, ~40 langs), Parakeet Flash (~250 MB, lowest-latency English), Parakeet TDT v3 (25 langs) & v2 (English), Cohere Transcribe (14 langs), Apple Speech (built-in, zero download), Whisper Tiny→Large (99 langs; the only Intel-Mac path) [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- **Fluid Intelligence**: separate privately-maintained local AI runtime (~3.5 GB model) for on-device dictation enhancement — smart formatting, context-aware capitalization, post-processing — "No cloud, no API keys, no data leaving your Mac"; explicitly NOT open source ("keeping Fluid Intelligence private… may change in the future") [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Command Mode: control the Mac by voice — launch apps, run shortcuts, trigger system actions, automate workflows [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Write Mode: dictate into or rewrite selected text in any app's text field via accessibility APIs ("Smart Typing" direct insertion) [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Optional cloud AI enhancement post-processing via OpenAI/Groq/custom providers; keys stored in macOS Keychain; strictly opt-in alongside the local Fluid Intelligence path [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Audio History: optional local recording history with budget controls and ZIP export for reviewing past dictations without cloud storage [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Per-app configuration: different prompt sets assigned to different target apps so enhancement adapts per workspace [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Privacy posture: "local-first… unless you explicitly opt in to a cloud AI provider"; anonymous analytics **enabled by default** (app version, config flags, approximate usage ranges; explicit no-voice/no-text list) with opt-out in Settings [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Licensing split: app GPLv3 since 2026-02-23 (Apache-2.0 before); Fluid Intelligence runtime proprietary/closed [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]
- Platform scope: macOS 15+ today, Apple Silicon required for all non-Whisper models; iOS and Windows waitlisted only [VERIFIED-fetched:https://raw.githubusercontent.com/altic-dev/FluidVoice/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| On-device dictation as an owner-input channel | 🏛 | Owner Console candidate (§16.10 input surface) / Edge & Device Ops pack family | Voice-to-prompt is an owner-device capability, not agent infrastructure: execution happens on the owner's Mac, outside the Linux boundary but also outside any cloud. Landing is the owner-input surface spec plus an Edge/Device pack reference build. |
| Multi-engine STT switcher with size/language/hardware matrix | 🧩 | §17.3 hardware-tiered model packs + §14.4 | The cleanest published example of engine-per-tier selection (Parakeet Flash vs Nemotron vs Whisper-for-old-hardware) — directly reusable as LastHarness's STT tier table for llama-swap-adjacent speech services. |
| Fluid Intelligence proprietary local enhancement runtime | ❌ | — | A closed binary executing on-device with undisclosed weights/architecture: locality passes but auditability fails; LastHarness cannot verify what a private runtime does to transcribed text. Replace with a local small-model formatting pass. |
| Cloud AI enhancement (OpenAI/Groq/custom) | ❌ | — | Voice content executed on third-party cloud when enabled; violates local-first + default-deny egress. Keep permanently disabled in-boundary. |
| Command Mode (voice → system actions) | ❌ | — | Direct voice-triggered OS control bypasses deny-by-default tool gating; the adoptable pattern is voice→queued command through the kernel approval gate (§16.10), never direct execution. |
| Write Mode accessibility text insertion | 🧩 | §16.10 approved-output channel | Text-insertion-via-accessibility is a viable owner-device output mechanism if gated as an approved action rather than ambient capability. |
| Per-app prompt-set configuration | 🧩 | context-profile pattern, §16.6 dispatch contexts | Target-app-aware enhancement prompts = per-surface context profiles; maps to dispatch-context customization. |
| Local Audio History with budget controls + ZIP export | 🧩 | retention-policy template, Security §14.7 | Budget-capped local retention with bulk export is a sane default for any audio LastHarness keeps; no cloud copy exists to leak. |
| Keychain key storage | 🧩 | secret-handling precedent | Platform-secret-store over plaintext config; LastHarness equivalent is kernel-held credentials never exposed to workers. |
| Default-on anonymous analytics | ⚙ | Security §14.7 install checklist | Opt-out-not-opt-in telemetry from a "local-first" app — disable at deployment; harmless payload but contradicts the posture LastHarness standardizes on. |

## Notable absences
No Linux or Windows build (waitlist only) — nothing here can execute inside the one-Linux-env boundary as shipped; no CLI/SDK/API for piping transcripts into other tools programmatically (closed pipeline end-to-end); no speaker diarization; no wake-word/hands-free mode documented; Fluid Intelligence model architecture and training data undisclosed; "fastest Parakeet on Mac" claim carries no benchmark methodology; GPLv3 (post-2026-02-23) matters only if redistributed — internal use unrestricted.
