# 04 · Local Inference & Runtimes

LLM runtimes, servers, and local-model tooling. Directly adjacent to LastHarness's locked V1 stack (§16.1: `llama-server` + `llama-swap`, one resident model, alias-based routing) — these are the alternatives/neighbors evaluated against it.

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #57 | LM Studio | Desktop app plus CLI for running local LLMs with OpenAI-compatible server | Res,Intel | ok | high | https://lmstudio.ai | https://github.com/lmstudio-ai/lms |
| #69 | local.ai | Independent local-AI benchmark site: models, hardware, energy | Res,Intel | no-repo(web-only) | high | https://local.ai | - |
| #70 | LocalStudio.ai | Desktop app unifying local model runtimes behind one OpenAI-compatible surface | Res,Dev | ok | high | https://localstudio.ai | https://github.com/sybil-solutions/local-studio |
| #103 | Lemonade AI | Local LLM server/runtime optimized for Windows NPUs (AMD-backed) | Dev,Intel | ok | med | https://lemonade-server.ai | https://github.com/lemonade-sdk/lemonade |
| #106 | LocalAI | Self-hosted OpenAI-compatible local LLM inference server | Dev,Intel | ok | high | https://localai.io | https://github.com/mudler/LocalAI |
| #134 | Ox Alpha | Stealth frontier reasoning model via OpenRouter/Zen routes; anonymous provider | Intel | no-repo(closed) | high | https://oxalpha.io | https://github.com/ox-alpha-installer/ox-alpha-installer |
| #144 | 9router | Free-token router with auto-fallback across providers for coding-agent CLIs | Intel | ok | high | - | https://github.com/decolua/9router |
| #118 | Roboflow RF-DETR | Real-time object detection/segmentation model architecture from Roboflow | Intel,Product | ok | high | https://rfdetr.roboflow.com | https://github.com/roboflow/rf-detr |
| #120 | FluidVoice | macOS offline dictation app with multi-model STT and optional AI enhancement | Product | ok | high | https://altic.dev/fluid | https://github.com/altic-dev/FluidVoice |
| #121 | Needle 2 | 14MB agentic tool-calling LLM for tiny devices; confidence-gated escalation | Intel,Kernel | ok | high | https://cactuscompute.com/needle | https://github.com/cactus-compute/needle |
| #117 | Yap | macOS on-device dictation via Apple SpeechAnalyzer; zero network, no model download | Product | ok | high | https://frigade.com/yap | https://github.com/FrigadeHQ/yap |

**Flags & notes**
- **#70/#71 ecosystem**: LocalStudio.ai and KittyLitter (01-coding-agents) are companion products by the same maker — Studio unifies runtimes, KittyLitter is its phone client for driving coding agents.
- None of these displace the §16.1 stack; LM Studio/LocalAI matter as Path B custom endpoints (user-supplied), which §16.1 explicitly supports via the narrow proxy exception.
- #69 is a benchmark/reference site (no product repo) — useful for hardware-tier pack sizing data (§17.3)
- **#134 Ox Alpha**: anonymous-provider stealth preview — route-dependent retention (OpenRouter retains prompts/completions; Zen zero-retention claim), no public weights, preview pricing temporary. Path-A evaluation option ONLY; never default inference — useful for hardware-tier pack sizing data (§17.3).
