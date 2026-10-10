# Ox Alpha — Feature Dossier
Registry: #134 | Category: 04 | Verified: no model repo (weights closed, provider anonymous); OpenRouter listing `stealth/ox-alpha` released 2026-08-20; community installer `ox-alpha-installer/ox-alpha-installer` live; specs corroborated by 3+ independent sources fetched 2026-08-25

## Feature inventory
- Free stealth frontier-class reasoning model for coding and sustained agentic work; $0/$0 pricing during preview [VERIFIED-fetched:https://openrouter.ai/stealth/ox-alpha via explainx.ai + wavect.io corroboration]
- 1,048,576-token context window; up to 131,072 output tokens [VERIFIED-fetched:https://explainx.ai/blog/openrouter-ox-alpha-stealth-model-august-2026]
- Native multimodal input: text + image + video in; text out [VERIFIED-fetched:https://felo.ai/tools/ox-alpha]
- Tool calling with tool_choice + structured output (no strict JSON-schema enforcement listed) [VERIFIED-fetched:https://wavect.io/blog/ox-alpha-free-ai-model-guide-2026/]
- Route-dependent retention: OpenRouter route retains prompts/completions (no training); OpenCode Zen route (`x-preview-f-free`) claims zero retention [VERIFIED-fetched:https://wavect.io/blog/ox-alpha-free-ai-model-guide-2026/]
- No public weights; developer identity undisclosed ("stealth" program) [VERIFIED-fetched:https://openrouter.ai/terms/stealth via wavect.io]
- Community one-click installer wiring the model into Claude Code / VS Code [VERIFIED-fetched:https://github.com/ox-alpha-installer/ox-alpha-installer]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Model-as-Path-A evaluation option (alias behind llama-swap routes) | ⚙ Feature adoption (conditional) | §17.2 Path A opt-in cloud inference; §16.1 alias routing | Fits the existing opt-in cloud pattern IF the owner accepts the retention posture; never a default |
| Anonymous-provider stealth distribution | ❌ Conflict-skip (as default) | §2.1 self-hosted/private posture; §13 attribution | No identifiable operator, no pinnable weights, route-dependent retention — supply-chain/attribution risk bars default use |
| Zero-retention Zen route | 🧩 Mechanism only | §13.4.2 gateway policy vocabulary | Route-level retention differentiation is the right control axis; LastHarness's egress allow-list should record per-route retention posture |
| 1M-context long-horizon work | 🧩 Mechanism only | Context economy service (§7.1 compaction protocol) | Large windows reduce but do not eliminate compaction need; corroborates R3 |
| Preview pricing ($0) | ❌ Conflict-skip as budget assumption | §14.4 cost policy | Free is a preview state, not a durable cost input for Intelligence planning |

## Notable absences
No self-host path whatsoever (weights withheld). No identified lab → no security-response channel, no roadmap accountability. Strict JSON-schema output enforcement not offered. Registry placement note: filed under 04-local-inference despite being cloud-only because it is a *model option* adjacent to the alias-routing layer, not an agent/harness; dossier records the tension explicitly.
