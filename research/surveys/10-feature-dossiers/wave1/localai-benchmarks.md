# local.ai — Feature Dossier
Registry: #69 | Category: 04 | Verified: unverified — no public repo; benchmark data produced by Exo Labs; site in "Early access" (site fetched 2026-08-23)

Site-content inventory, not a code harvest: local.ai is a closed benchmark SITE comparing complete local-AI setups (model × harness × hardware × inference engine × configuration × quantization). No repo, no API, no downloadable dataset found on fetched pages.

## Feature inventory
- Setup-level comparison thesis: scores the whole combination (model, harness, hardware, inference engine, runtime config such as speculative decoding, quant/pruning variant) instead of any single layer [VERIFIED-fetched:https://local.ai/about]
- Anti-metric argument, backed by paired charts: near-identical decode throughput (17.77 vs 17.14 tok/s @32K ctx) yields 3× different GDPVal task time; neither FP32 TFLOPS nor memory bandwidth predicts end-to-end task time [VERIFIED-fetched:https://local.ai/about]
- Quant-aware evaluation: Ornith 397B IQ2_XXS ≈ Ornith 35B Q4_K_M on GDPVal score while costing 5× disk / 11× parameters — exact variation is tested, model name alone is not a result [VERIFIED-fetched:https://local.ai/about]
- Models leaderboard: 42 models, 171 scored variants; Intelligence Index = 2% × τ²-bench + 36% × GAIA + 62% × GDPval; per-model τ²/GAIA/GDPval columns; community ▲▼ votes [VERIFIED-fetched:https://local.ai/models]
- Speed defined end-to-end: "Tasks per Hour" from median real-workload task time; tok/s reported as measured decode/prefill at cold 32k-token context, single stream; best measured run shown whole-percent [VERIFIED-fetched:https://local.ai/models]
- Hardware catalog: 10 devices with price + unified-memory flag (DGX Spark 128GB $4.7k, M3 Ultra 96GB $5.3k in two GPU configs, M4 Max/M4 Pro, M5 Max 128GB $6.7k/M5 Pro, RTX 4090/5090, RTX 6000 Ada 48GB, RTX PRO 6000 Blackwell 96GB $15.5k) [VERIFIED-fetched:https://local.ai/hardware]
- Intelligence/speed Pareto frontiers per hardware device plus global budget frontiers at price thresholds ($2.7k–$15.5k slider) [VERIFIED-fetched:https://local.ai/hardware]
- Cost dimension includes estimated electricity where measurement supports it; dedicated Energy nav section (/energy) [VERIFIED-fetched:https://local.ai/about for cost method; /energy existence via nav — content unfetched]
- Independence statement: speed/eval data produced by Exo Labs, no commercial relationship with vendors; missing measurements stay missing [VERIFIED-fetched:https://local.ai/about]
- Community surfaces: Wall (/wall), referrals program, claim-your-name accounts, "email me when this data changes" watchers [VERIFIED-fetched:https://local.ai + /hardware]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Setup-level benchmark (model×quant×engine×hardware→task outcome) | ⚙ Feature adoption | Local inference dept — model-pack preset selection (Step 0/4 hardware-tiered packs) | Exactly the decision LastHarness asks at install: pick device-appropriate presets per category; adopt the methodology, consume figures offline |
| Task-time > tok/s as the primary speed metric | 🧩 Mechanism only | Pack benchmark harness § | Evaluation-methodology pattern: benchmark packs by end-to-end task time on representative workloads, not throughput alone |
| Weighted Intelligence Index over multiple evals (τ²/GAIA/GDPval) | 🧩 Mechanism only | Pack quality scoring § | Transparent weighted-median scoring formula reusable when ranking which quant fills a pack slot |
| Budget Pareto frontiers per hardware | 🧩 Mechanism only | Hardware recommendation logic § | Frontier-at-price-threshold presentation is the right shape for advising owners what their box can run |
| Electricity-cost estimation in comparisons | ⚙ Feature adoption | Observability/reporting § | Cheap to compute locally from measured watts-equivalent; supports owner cost reports |
| Community votes / Wall / referrals / claim-your-name | ❌ Conflict-skip | — | Multi-user community product surfaces; single-owner kernel has no audience layer |
| Live site as data source | ❌ Conflict-skip | Reference-only citation | Third-party web service; figures may be cited in docs/pack notes but nothing executes remotely |

## Notable absences
No public repo, no API, no dataset download, no reproducible benchmark harness — figures are consume-only and unauditable. Early-access gating on some surfaces. Hardware set is Apple-unified-memory + NVIDIA GPUs only; no AMD, no CPU-only tiers. Energy section content not yet measured everywhere (site says missing measurements stay missing). No per-agent-framework (harness) leaderboard detail on fetched pages despite harness being one of the six compared axes. Nothing installable — LastHarness can adopt methodology and cite results, never ship the thing itself.
