# Magnitude — Feature Dossier
Registry: #56 | Category: 01 | Verified: 1,504 stars, Apache-2.0, pushed 2026-08-23 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: hardware-profile→pack flow = Appendix A Steps 0/4 exact. The README confirms it feature-for-feature, and Magnitude is one of the three independent arrivals (#56/#57/#70) the registry already cites as local-first convergence evidence.

## Feature inventory
- **Hardware profile → recommend → download → run inside the agent**: "it profiles your hardware, recommends and downloads models, then configures and runs them inside the agent. Nothing else to set up." Positioned against Ollama ("runs local models") and Hermes ("an agent that can use local models") as combining both [VERIFIED-fetched:https://raw.githubusercontent.com/magnitudedev/magnitude/main/README.md]
- **Fully private and offline**: "Everything stays on your machine, including the models"; after download it runs "without an internet connection"; "Your prompts and files stay on your machine" [VERIFIED-fetched:https://raw.githubusercontent.com/magnitudedev/magnitude/main/README.md]
- **No fixed hardware minimum**: "Magnitude profiles your hardware and recommends the best models for your machine. More memory lets you run larger models." [VERIFIED-fetched:https://raw.githubusercontent.com/magnitudedev/magnitude/main/README.md]
- **Out-of-catalog GGUF**: compatible GGUF models downloadable from Hugging Face beyond the built-in catalog [VERIFIED-fetched:https://raw.githubusercontent.com/magnitudedev/magnitude/main/README.md]
- **Custom OpenAI-compatible endpoint**: you can connect your own inference server and use its models in Magnitude [VERIFIED-fetched:https://raw.githubusercontent.com/magnitudedev/magnitude/main/README.md]
- **Skills via cross-harness skill packs**: `npx skills add` from the skills.sh directory — recommends vercel-labs/agent-browser and anthropics/skills xlsx/pptx/docx/pdf [VERIFIED-fetched:https://raw.githubusercontent.com/magnitudedev/magnitude/main/README.md]
- Platform support: macOS and Linux native, Windows through WSL; Apache-2.0; npm-distributed CLI (`@magnitudedev/cli`) [VERIFIED-fetched:https://raw.githubusercontent.com/magnitudedev/magnitude/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Hardware scan → model recommendation → download → configured runtime | 🧩 Mechanism only | Validates Appendix A Step 0 + Step 4 exact flow | The recorded skip confirmed step-for-step: silent hardware detection assigning a tier (Step 0), then pack download with per-device configuration (Step 4's llama-bench-maxx → `llama-swap.yaml`). Third independent arrival at LastHarness's exact onboarding shape. |
| Runtime bundled so no Ollama/model-server setup exists | 🧩 Mechanism only | Validates §16.1 bundled llama-server/llama-swap decision | LastHarness made the same call — ship the inference runtime as part of the app rather than requiring a separately installed model server. |
| Offline-after-download operation | 🧩 Mechanism only | Validates local-first inference invariant | Confirms the pack-download-then-work-air-gapped posture is achievable and is a selling point competitors advertise. |
| Custom OpenAI-compatible endpoint connect | 🧩 Mechanism only | Mirrors Path B user-supplied endpoints (Appendix A Step 1; §16.1 Path B proxy exception) | Same escape hatch for owners who already run a server; LastHarness's adapter scope already covers this narrow case. |
| Cross-harness skill-pack consumption (`npx skills add`) | 🧩 Mechanism only | Validates §14.5 plain-Markdown skill-pack portability | Wave-1 finding reconfirmed: plain-Markdown skills are harness-portable, so LastHarness's skills registry can draw from the same ecosystem. |
| Whole-tool adoption as LastHarness's agent | ❌ Conflict-skip | — | Magnitude-the-harness is another agent runtime — excluded substrate under §16.2 regardless of how well its onboarding matches Appendix A. |

## Notable absences
No permission system, approval gates, or egress policy mentioned. No memory/knowledge lifecycle. No benchmark sweep documented at README level — recommendation criteria ("profiles your hardware") are not quantified the way Step 4's measured tokens/sec table is. No multi-model simultaneous residency or alias routing story (single bundled runtime). No team/owner-console surface — single-user CLI product.
