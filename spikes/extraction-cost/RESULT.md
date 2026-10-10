# Spike 5: cost of the end-of-session extraction turn on a local model

Question: when the session ends and the harness asks the model "what is worth remembering?", how long does the model take to read the conversation again, and how much do prefix caching and slot save/restore help?

How: `bench.py` starts its own `llama-server` (one slot, GPU as in the llama-swap config, `--cache-ram 0` so no hidden host cache), builds a synthetic chat of the target size, and asks the extraction question as a final user turn with `max_tokens: 1`, so the timing is the reading of the prompt (prefill) and not the answer. Server numbers (`timings.prompt_ms`, `prompt_n`, `cache_n`) are quoted. `run_all.sh` has the commands. Each case ran twice, and both runs agreed to within a few percent.

Cases:
- **A cold:** nothing cached. The whole session plus the question is read.
- **B cache hit:** the session was already read during the chat (the cache holds it), then the question is added.
- **C restore:** the slot was saved to disk right after the session, wiped (as if another job used the server, or it unloaded), restored from disk, then the question is added.
- **D prefix changed:** an earlier part of the prompt differs (a new system line), so the cache cannot be reused. This is what a compaction or an edit to early turns would do.

Engine: prebuilt llama.cpp b11541 (CUDA build, in `~/AI/engines/_prebuilt/mainline-b11541`).

| Model | Size of chat | A cold | B cache hit | C restore | D prefix changed | Slot file / save / restore |
|---|---|---|---|---|---|---|
| LFM2.5-2.6B (Q4_K_M) | 5,200 tokens | 1.8 s | 0.41 s (1,054 tokens re-read) | 0.41 s | 1.8 s | 44 MB / 0.03 s / 0.01 s |
| Spark-X2.5-4B (Q4_K_M) | 5,200 tokens | 3.2 s | 0.16 s (120 tokens re-read) | 0.20 s (169 re-read) | 3.2 s | 212 MB / 0.14 s / 0.05 s |
| Ornith-1.5-35B-A3B NanoPlus (all experts on CPU) | 2,700 tokens | 8.7 s | 0.8 s (31 tokens re-read) | 0.75 s | 8.3 s | 342 MB / 0.2 s / 0.17 s |

Reading speed when cold: about 2,900 tokens/s (LFM), 1,650 (Spark), 315 (Ornith 35B with all experts on the CPU; the user's own gs engine with its expert cache is faster, but it was not used here).

## What it says

- Cold extraction is linear in the session length. At the Ornith 35B speed, a 20,000-token session would take about a minute to read. At the small models' speed it takes 7 to 12 s. That is tolerable for a small model and noticeable for a big one.
- Keeping the prompt append-only, with the question as the last turn, works: the cost drops 4x (LFM), 20x (Spark) and 10x (Ornith). The cache hit is the main saver, as the spec assumed.
- LFM2.5 re-reads about 1,050 tokens even on a hit. It is a hybrid model with a recurrent part that cannot be rewound a few tokens, so the server rolls back to the last saved checkpoint. The saving is 78%, not 98%.
- Slot save/restore gives nearly the same result as a live cache hit: 0.2 to 0.75 s, with a slot file of 44 to 342 MB that writes and reads in well under a second. So a session can be parked on disk and extracted later (for example when idle, after the model was unloaded) without paying the cold cost. A 20,000-token session will make a bigger file (roughly 2.5 GB for Ornith at the observed 0.13 MB per token, 0.8 GB for Spark at 0.04 MB per token), so old slot files need a size limit and clean-up.
- Case D shows the risk: any change to earlier text (new system line, edited turn, compaction that rewrites the start) throws the whole benefit away and costs the full cold price. Compaction summaries must be appended, as the spec says, and the system prompt must stay byte-identical across the session.

## Important finding on the engine

On the older local build (llama.cpp build 287, in `llama-mainline`), restore worked (tokens restored) but the next request ignored it and read everything again (cold price, tested on LFM and Spark). On the b11541 prebuilt it works. So the capability probe in the provider layer should test restore with a real request, not trust the endpoint's presence, and the app should fall back to the cold path without error.

## Limits

- Synthetic text from a small vocabulary; token counts are real, content is not. Prefill speed does not depend on content, but generation of the actual extraction answer was not measured (one token only). Answer time depends on the model's thinking and is a separate cost (thinking models spent 100 to 1,000 tokens in spike 1).
- Only 5,000 tokens for the small models and 2,700 for the 35B. Longer sessions grow linearly for the cold case, and the slot file grows too.
- The 35B was run with all experts on the CPU, a different setup from the user's llama-swap entry.
- `llama-swap` and its `slotkeeper` wrapper were bypassed. Nothing here says how they behave with extraction.
- Each server was started by the script and stopped afterwards. llama-swap's own model was unloaded first and loads again on the next request.
