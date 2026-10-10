# LocalStudio.ai — Feature Dossier (THIN-SOURCE)
Registry: #70 | Category: 04 | Verified: registry row has NO repo ("-" recorded); official site localstudio.ai only. Site links a GitHub repo (sybil-solutions/local-studio): 1,738 stars, Apache-2.0, TypeScript, last push 2026-08-22 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: single-endpoint facade ≈ llama-swap aliases. THIN-SOURCE: registry evidence base is the marketing site; extracted accordingly, not padded.

## Feature inventory
- **Positioning**: macOS desktop app — "Run your intelligence at home… AI workspace built to help you go local"; pitch keeps "the model, the runtime, and the hardware together. Nothing important disappears behind a provider abstraction" [VERIFIED-fetched:https://localstudio.ai]
- **Serve = one OpenAI-compatible facade over multiple runtimes**: "vLLM, SGLang, MLX, and llama.cpp behind one OpenAI-compatible surface" [VERIFIED-fetched:https://localstudio.ai]
- **Control panel**: "Local and remote controllers, live status, launch state, logs, and metrics"; screenshots show proxied tokens/requests/sessions usage views [VERIFIED-fetched:https://localstudio.ai]
- **Workbench**: models, providers, browser, files, terminal, and agents "in the same session"; model configuration screen shows searchable Hugging Face catalog with hardware-fit and downloads [VERIFIED-fetched:https://localstudio.ai]
- **KittyLitter companion**: pair your phone from Local Studio to drive Codex, Claude, OpenCode, Pi, Droid remotely (kittylitter.app) [VERIFIED-fetched:https://localstudio.ai]
- **Download offered for macOS only** on the landing page; GitHub link points to sybil-solutions/local-studio whose repo description reads "Control panel for VLLM, Sglang, llama.cpp, exllamav3" (exllamav3 appears only there, not in site copy) [REPORTED-unverified:https://localstudio.ai + VERIFIED-fetched:https://api.github.com/repos/sybil-solutions/local-studio]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| One OpenAI-compatible endpoint over vLLM/SGLang/MLX/llama.cpp | 🧩 Mechanism only | Validates existing llama-swap aliases in §16.1 | The recorded skip confirmed feature-for-feature: alias-routing many runtimes behind one endpoint is already LastHarness's Path B facade via llama-swap; LocalStudio is the same mechanism with a desktop GUI attached. |
| Launch-state control + proxy usage metrics (tokens/requests/sessions) | 🧩 Mechanism only | §16 Owner Surfaces display pattern | Live status/logs/per-endpoint usage attribution is the right owner-console shape; kernel already holds these counters — only the presentation transfers. |
| HF-catalog model browser with hardware-fit hints | 🧩 Mechanism only | Adjacent to Step 0 hardware-tiered packs | Searchable-models-plus-hardware-fit mirrors LastHarness's pack-selection flow for non-technical onboarding; pattern reference only. |
| Desktop app workbench (browser/files/terminal/agents in-session) | ❌ Conflict-skip | — | A general agent-workspace product surface duplicates what LastHarness deliberately does not build; Personal Agent owns any chat/work UX. |
| Remote controller + KittyLitter phone pairing | ❌ Conflict-skip | — | Remote-control channel over network to drive coding agents expands the attack surface beyond the single local boundary; outside default-deny egress discipline. |

## Notable absences
THIN-SOURCE held: marketing-site copy is nearly all the public documentation surfaced here; no pricing/self-host story, no docs depth pulled into the registry, no Windows/Linux builds advertised (macOS download only). Registry recorded NO repo while the site now links sybil-solutions/local-studio (1,738★, Apache-2.0) — a registry-correction candidate NOT applied here (out of write scope). exllamav3 support is visible only in the repo description, unconfirmed by site docs. No stated position on multi-user access, egress, or telemetry. Nothing about memory, logging durability, or onboarding flows.
