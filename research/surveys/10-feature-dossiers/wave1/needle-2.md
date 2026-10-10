# Needle 2 — Feature Dossier
Registry: #121 | Category: 04 | Verified: Apache-2.0 (license badge on HF model card); 45M params, 14MB binary, ~28MB session RAM per README; repo cactus-compute/needle + HF Cactus-Compute/needle2; star count unverified (191 HF likes, ~28.7k downloads/mo on HF)

## Feature inventory
- 45M-parameter tool-calling/device-use/extraction model delivered as a single self-contained 14MB engine binary (weights baked in, CQ2-bit "Cactus Quants" compression); inference does no network, air-gapped install documented [VERIFIED-fetched:https://raw.githubusercontent.com/cactus-compute/needle/main/README.md]
- Grammar-constrained decoding: a byte-level grammar compiled from declared JSON schemas constrains every token, so tool calls cannot be malformed; `needle.Field` constraints (ranges, patterns, lengths, enums) compile into the same grammar [VERIFIED-fetched:https://huggingface.co/Cactus-Compute/needle2]
- Confidence gating: every response carries a calibrated confidence = min(post-hoc head score, decode probability of call tokens); contract is act above threshold, escalate/re-ask below [VERIFIED-fetched:https://raw.githubusercontent.com/cactus-compute/needle/main/README.md]
- Tool retrieval head: for large catalogues only the top-5 scoring tools enter context per turn and grammar is rebuilt over that subset — an unselected tool is unreachable, not unlikely; embeddings persistable to a fingerprint-keyed `.idx` [VERIFIED-fetched:https://huggingface.co/Cactus-Compute/needle2]
- Bounded memory: 256-token sliding window with tools pinned as KV sinks keeps session RAM ≈28MB regardless of conversation length [VERIFIED-fetched:https://raw.githubusercontent.com/cactus-compute/needle/main/README.md]
- Empty-call refusal contract: off-topic requests return `[]` (no free-text fallback); optional fields without evidence are omitted, not guessed [VERIFIED-fetched:https://huggingface.co/Cactus-Compute/needle2]
- System-facts turn: environment state (date/locale/device/battery) passed as facts, never instructions; instructions placed there do not steer the model — relative phrases resolve only when facts license them [VERIFIED-fetched:https://huggingface.co/Cactus-Compute/needle2]
- Structured extraction API: declare schema (Pydantic model or raw JSON) as sole tool → guaranteed-conforming extraction, no separate JSON mode; `run()` loop executes tools and feeds results back, multi-step calls can depend on earlier results [VERIFIED-fetched:https://raw.githubusercontent.com/cactus-compute/needle/main/README.md]
- LoRA fine-tuning pipeline with optional LLM-synthesized data (`needle generate-data`), merged-at-export tuned `.cact` artifacts running unchanged on the same engine; JAX training on CUDA/Metal [VERIFIED-fetched:https://raw.githubusercontent.com/cactus-compute/needle/main/README.md]
- Per-platform engines: macOS/Linux x86-64/ARM64/ARMv7/RISC-V/MIPS, Windows, Android, iOS/watchOS/tvOS static lib, WebAssembly (`needle.js`+`.wasm`), plus CLI mode and localhost HTTP `--serve` mode; ~500 tok/s on Raspberry Pi 5 [VERIFIED-fetched:https://huggingface.co/Cactus-Compute/needle2]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Needle model as low-tier hardware pack member | ⚙ Feature adoption | §model-packs (lowest tier) | Local-first inference on CPU-class/self-hosted Linux hardware; execution stays in LastHarness boundary |
| Grammar-constrained tool calls from schemas | ⚙ Feature adoption | §tool-invocation safety | Structurally impossible malformed calls = strongest deny-by-default form for function calling |
| Confidence threshold → escalate | 🧩 Mechanism only | §routing policy | Maps to kernel-mediated routing: low confidence escalates to bigger local tier, never outward |
| Top-5 tool retrieval with hard unreachability | 🧩 Mechanism only | §tool allow-lists | Dynamic allow-list generation is exactly the allow-list-only posture at model level |
| Empty-call `[]` refusal contract | 🧩 Mechanism only | §prompt/response contract | Deny-by-default expressed in the model's output protocol |
| System-facts instruction-resistant context channel | 🧩 Mechanism only | §context injection | Prompt-injection-resistant way to feed env state to small models |
| KV-sink pinned tools + sliding window | 🧩 Mechanism only | §inference runtime | Constant-memory long sessions on tiny hardware |
| LoRA fine-tune → single-file `.cact` export | ⚙ Feature adoption | §model-pack build pipeline | Per-department tuned packs as portable artifacts; training happens locally |
| WASM/browser engine target | ❌ Conflict-skip | — | No browser runtime inside LastHarness Linux boundary |

## Notable absences
No general chat/reasoning capability — it only emits tool calls/extractions, so it cannot replace a general assistant tier; no streaming partial outputs; 256-token window excludes long-document reasoning; no built-in auth on its localhost HTTP server; ecosystem/benchmarks are vendor-reported.
