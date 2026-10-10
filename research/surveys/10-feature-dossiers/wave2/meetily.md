# Meetily — Feature Dossier
Registry: #119 | Category: 07 | Verified: 29,807 stars, MIT, last push 2026-06-05

## Feature inventory
- Privacy-first meeting assistant: capture, live-transcribe, and summarize entirely on local infrastructure; "no data ever leaves your computer" positioning for compliance-sensitive users [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- Local real-time transcription with **Whisper or Parakeet** models on-device ("4x faster" Parakeet/Whisper claim in tagline); credits whisper.cpp, screenpipe, transcribe-rs, and NVIDIA Parakeet via istupakov's ONNX conversion [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- Flexible summary providers: **Ollama (local) recommended**, plus Claude, Groq, OpenRouter, or any custom OpenAI-compatible endpoint — explicitly "perfect for organizations with custom AI infrastructure" [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- Import & Enhance (Beta): import existing audio files to generate transcripts, or re-transcribe any recorded meeting with a different model or language — all processed locally [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- Professional audio mixing: simultaneous microphone + system-audio capture with intelligent ducking and clipping prevention [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- GPU-accelerated builds per platform: macOS Apple Silicon (Metal) + CoreML; Windows/Linux NVIDIA (CUDA) and AMD/Intel (Vulkan); auto-enabled at build time [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- Cross-platform desktop app: Windows installer, macOS dmg, Linux build-from-source (`build-gpu.sh`); Tauri single self-contained app — Rust backend for core logic, Next.js frontend [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- Local data sovereignty: transcription models, recordings, and transcripts all stored locally [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]
- Meetily PRO (closed, separate codebase): enhanced-accuracy models, custom summary templates, PDF/DOCX/Markdown exports, auto-meeting detection/join, self-hosted team deployment, GDPR audit trails; speaker diarization, chat-with-meetings, calendar integration listed as coming to PRO; Community Edition stays free/MIT forever [VERIFIED-fetched:https://raw.githubusercontent.com/Zackriya-Solutions/meetily/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| End-to-end local meeting capture→transcript→summary | 🏛 | Meeting-Ops dept candidate (or Automation §14.6 capability if not spun out) | A whole business function serving every department's records; execution is fully in-boundary (local STT + Ollama). Candidate for a small dedicated blueprint rather than shoehorning into an existing dept. |
| Dual-engine local STT (Whisper / Parakeet swap) | 🧩 | §17.3 hardware-tiered model packs | Engine-per-language/latency/hardware selection is exactly the pack-tier mechanism; Parakeet-via-ONNX on Vulkan/CUDA maps to the Linux-env GPU matrix. |
| Ollama / custom OpenAI-compatible endpoint summaries | ⚙ | Intelligence §14.4 | Custom-endpoint support means llama-server drops in unmodified — the summarization step passes the execution litmus as-is. |
| Import & Enhance re-transcription | ⚙ | same Meeting-Ops home | Model-swappable re-processing of archived audio = reproducibility upgrade path without re-recording; fits decision-log replay thinking. |
| Mic + system-audio mixing with ducking/clipping control | 🧩 | capture-quality pattern for Meeting-Ops | The hard-won detail that makes transcript quality usable on real calls; pure mechanism, engine-agnostic. |
| GPU build matrix (Metal/CUDA/Vulkan/CoreML) | 🧩 | §17.3 pack build profiles | Build-time hardware detection enabling the right backend — template for LastHarness's per-hardware model-pack builds. |
| Tauri/Rust desktop app (Windows/macOS binaries, Linux source-only) | ⚙ | Meeting-Ops, headless re-host inside Linux boundary | Linux build-from-source exists so capture/transcribe can run in-boundary; the desktop GUI shell itself is out of scope for a headless agent environment. |
| PRO tier (closed codebase, paid) | ❌ | — | Closed-source fork whose headline features (diarization, exports, auto-join, audit trails) cannot be audited or self-verified; Community Edition supersedes for LastHarness. |
| GDPR/audit-trail claims (PRO marketing) | ❌ | — | Unverifiable in closed code; §12's immutable log is LastHarness's own answer and needs no closed component. |

## Notable absences
No speaker diarization in Community Edition (PRO-only roadmap item) — transcripts are single-stream; no calendar integration or auto-join in CE; no document export formats documented for CE beyond the editor (PDF/DOCX sold in PRO); no search across past meetings documented; no API/webhook surface so other departments could consume minutes programmatically; no retention/redaction policy tooling; Linux users get source builds only, no signed binaries.
