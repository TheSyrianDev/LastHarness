# Yap — Feature Dossier
Registry: #117 | Category: 04 | Verified: MIT (badge in README); macOS 26+ Apple Silicon only; official site frigade.com/yap linked from README; stars/last-push unverified

## Feature inventory
- On-device streaming dictation via macOS 26 `SpeechAnalyzer`/`SpeechTranscriber` system APIs: app ships no model, loads nothing before the first word, no API key, no per-minute cost [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]
- Zero-network guarantee: "no network code in the app at all"; refuses to use `SFSpeechRecognizer` because it can fall back to Apple servers — if `SpeechAnalyzer` can't handle a language on device, dictation stops rather than sending audio anywhere [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]
- Global shortcut (default ⌘⇧D), fully rebindable incl. single-modifier triggers (right shift); press-to-stop model with live waveform + partial transcript preview; double-Escape discards a take before insertion [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]
- Text lands in the previously focused app via clipboard-write → synthetic ⌘V → clipboard restore, with a deliberate wait before restore because Chromium apps read the pasteboard asynchronously [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]
- Local transcript history stored in SwiftData with search, copy, delete — re-copy anything dictated earlier [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]
- Pre-init audio buffering: capture starts before the speech stack attaches and buffered audio is flushed to the transcriber so the first word is never clipped; follows default-mic changes mid-session [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]
- Explicit permission table at first launch explaining each of 4 grants (Microphone, Speech Recognition, Accessibility, Automation); Accessibility must be enabled by hand in System Settings [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]
- Deliberately minimal footprint: ~3k lines native Swift, 4 MB app, ~60 MB idle RAM, no Electron/browser engine, no account or telemetry, signed+notarized releases, Homebrew cask install [VERIFIED-fetched:https://raw.githubusercontent.com/FrigadeHQ/yap/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Yap app itself (menu-bar dictation) | ❌ Conflict-skip | — | macOS-only native app outside self-hosted Linux boundary |
| OS-shipped speech models instead of bundled weights | 🧩 Mechanism only | §model-packs | Pattern: prefer platform-managed models when they meet local-only guarantees; informs hardware-tiered pack design |
| Refuse-the-feature over network fallback | 🧩 Mechanism only | §egress-gateway policy | Encodes "fail closed, never degrade into egress" as product behavior; mirrors default-deny gateway |
| Capture-before-ready buffering flush | 🧩 Mechanism only | §voice-capture pipeline | Solves first-word clipping for any future dictation input path |
| Press-to-stop with cancelable live preview | ⚙ Feature adoption | §onboarding/input UX | Cancel-before-commit interaction prevents bad inserts in terminals/agents |
| Clipboard-restore paste with async-pasteboard wait | 🧩 Mechanism only | §text-insertion mechanism | The one hard-won detail that makes cross-app insertion work; document as runbook |
| Permission-explanation table per capability | 🧩 Mechanism only | §onboarding | Non-technical onboarding: every permission paired with a why |
| Local searchable history of past transcripts | ⚙ Feature adoption | §dept owning voice/text input history | Small, local-first history store pattern consistent with gated memory |

## Notable absences
No custom-vocabulary/user-dictionary support; no language picker yet (follows system locale); no text post-processing/edit commands (punctuation, formatting); no Windows/Linux port and explicitly none planned; single-user desktop scope — nothing networked, nothing multi-tenant.
