# Lemonade — Feature Dossier
Registry: #103 | Category: 04 | Verified: 5,447 stars, Apache-2.0, last push 2026-08-24

SCOPE: local LLM/multi-modal server optimized for AMD NPUs/GPUs (AMD-sponsored), evaluated as the hardware-compat evidence base for the §17.3 "Below-Low tier" extension + §14.4 escalation-threshold policy proposal.

## Feature inventory
- Local AI server exposing standard **OpenAI, Anthropic, and Ollama** APIs at `http://localhost:13305/v1` so hundreds of existing apps attach without code changes [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Two flavors: **Lemonade Server** (installed service) and **Embeddable Lemonade** — a portable binary apps bundle for auto-optimizing multi-modal local AI with zero Lemonade installers/branding/telemetry [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Per-machine hardware-compat probe: `lemonade backends` lists exactly which engines/backends the current PC supports [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Generated backends matrix (`BEGIN GENERATED: backends-matrix`) mapping modality → engine → backend → device → OS: llamacpp across system/cuda (sm_75+)/vulkan/rocm/cpu; **FLM on XDNA2 NPU (Windows+Linux)**; ryzenai-llm XDNA2 NPU (Windows); whispercpp incl. NPU backend; kokoro TTS; sd-cpp image gen; trellis 3D and onnxruntime classification (experimental) [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Explicit hardware tables: AMD ROCm families gfx1151 (Strix Halo iGPU), gfx120X (RDNA4), gfx110X (RDNA3); NVIDIA compute-capability tiers sm_75 Turing → sm_100/sm_120 Blackwell [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Model zoo spanning GGUF, FLM, and ONNX plus whisper/stable-diffusion models across CPU/GPU/NPU; Model Manager GUI + `lemonade pull/list`; custom GGUF/ONNX pullable from Hugging Face or ModelScope with source retained for updates [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Model aliases with environment-independent naming and instant active-standby failover: second `alias add production-llm <model>` swaps the target with no client change [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Multi-modal CLI parity: `lemonade run` for text/SDXL-Turbo/kokoro-v1/Whisper-Large-v3-Turbo; `lemonade launch claude` wires Claude Code to local models [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Packaging breadth: Windows MSI, macOS, Docker, Snap, Arch/Debian/Fedora/Ubuntu PPA — real Linux-env coverage, not Windows-only despite NPU heritage [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Cloud Offload (experimental): route to OpenAI-compatible cloud providers alongside local models [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- Stated privacy posture: no transfer to networked systems unless requested; contacts Hugging Face Hub / ModelScope only for user-requested model downloads or registry lookups [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]
- App marketplace integrations incl. documented Claude Code, Open WebUI, AnythingLLM, OpenHands paths via marketplace PRs [VERIFIED-fetched:https://raw.githubusercontent.com/lemonade-sdk/lemonade/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Local inference server (OpenAI/Anthropic/Ollama-compatible APIs) | ⚙ | Intelligence §14.4 + §17.3 Below-Low tier candidate | In-boundary execution by design; the NPU-first engine coverage (FLM/RyzenAI on XDNA2) fills the gap llama-server has below the Low tier — a complement beside llama-server, selected per hardware tier, not a replacement of the locked default. |
| `lemonade backends` machine-capability probe | 🧩 | §17.3 hardware-capability discovery | Runtime self-interrogation ("what can THIS box run?") is the missing primitive for automatic model-pack selection on unknown Below-Low hardware. |
| Generated backends-matrix (modality×engine×device×OS) | 🧩 | §17.3 pack-manifest schema | A declarative compatibility table generated from truth, consumed by tooling — the exact shape LastHarness's tier→pack mapping needs instead of hand-maintained matrices. |
| NPU backends (flm, ryzenai-llm, whisper-npu) targeting XDNA2 | ⚙ | Intelligence §14.4 Below-Low packs | First-class NPU execution for LLM and STT extends local-first inference onto budget/NPU-only machines where GPU/CPU serving is impractical. |
| Alias active-standby failover | 🧩 | §14.4 model-pack failover semantics | Environment-stable names with hot-swappable targets give graceful degradation when a pack's primary model is unavailable on a weak device — mirrors llama-swap's role with simpler mechanics. |
| Embeddable zero-telemetry binary | 🧩 | §17.3 deployment form for constrained devices | Portable single-artifact embedding with no phoning home matches the no-third-party-runtime boundary and suits non-server Below-Low deployments. |
| GGUF/FLM/ONNX multi-format zoo w/ HF+ModelScope pull | ⚙ | Intelligence §14.4 pack sourcing | Format breadth means one server fronts whatever artifact each vendor-tier pack ships; registry-lookup egress is user-initiated only, compatible with default-deny policy as an explicit allow. |
| `lemonade launch claude` / Claude Code integration | ⚙ | §16.2-compatible local model wiring | Shows the harness-to-local-server join LastHarness uses: Claude Agent SDK workers pointed at localhost inference stay inside the SDK lock while satisfying local-first. |
| Cloud Offload routing | ❌ | — | Even experimental, it normalizes sending agent inference off-boundary; conflicts with local-first and the escalation-threshold policy must be the ONLY sanctioned escape hatch (§14.4), not a server feature. |
| Linux distro + Docker packaging breadth | 🧩 | §16.1/§17.2 deployment hygiene | Mainstream-distro packaging keeps deployment inside the ONE self-hosted Linux-env boundary without third-party runtime layers. |

## Notable absences
No documented resource-footprint floor (min RAM/NPU TOPS per model) — the Below-Low tier still lacks the hard numbers needed for pack gating. No multi-tenant/auth story for the local API (localhost trust assumed), fine behind the boundary but unspecified. Failover is manual alias swapping, not health-triggered automatic failover like llama-swap. Cloud Offload exists at all and must be configuration-banned in LastHarness builds. Mobile companions exist but are separate repos not assessed here.
