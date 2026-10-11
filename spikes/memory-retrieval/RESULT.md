# Spike 3: memory retrieval, keyword only versus keyword plus a small embedding model

Question: does a small built-in embedding model beat keyword search for memory, in English and Arabic, and which model?

## How it was tested

`bench.py` (stdlib Python) builds one corpus of 230 short texts and asks 34 questions:
- 10 gold memories, each written in English and in Arabic by hand (20 texts). They cover a sad event, style preferences, a database decision, an allergy, money, a trip, hardware, a hobby, a backup decision and a family birthday.
- 60 written filler texts (30 English, 30 Arabic) and 150 real English chunks taken from this machine's own Claude Code transcripts, as distractors. Those chunks stay local and are never printed.
- Questions: lexical (one word that appears in the memory), paraphrase (same meaning, almost no shared words), and cross-language (question in one language, memory in the other). 16 English, 16 Arabic (6 lexical and 10 paraphrase each), 4 cross-language.
- Score: is the right memory in the top 5 (hit), and mean reciprocal rank (mrr, 1.00 is always first).
- Keyword search is BM25 with light Arabic normalisation (diacritics, alef forms, common prefixes) and a small stopword list. Embedding search is cosine similarity. Hybrid is reciprocal rank fusion (equal weights, keyword list limited to documents that share a word).
- Embeddings come from `llama-server --embedding` on the CPU (6 threads, Q8 files). Engine: prebuilt llama.cpp b11541 (CUDA build, run with `-ngl 0`), downloaded into `~/AI/engines/_prebuilt/mainline-b11541`. The older local builds could not load EmbeddingGemma 2 (`unknown model architecture: 'gemma-embedding2'`).
- Prompts: EmbeddingGemma uses its documented `task: search result | query: ` and `title: none | text: ` prefixes, e5 uses `query: ` and `passage: `, BGE-M3 uses none.

## Results (top-5 hits out of N, mrr)

| Method | lexical EN | paraphrase EN | lexical AR | paraphrase AR | cross-language |
|---|---|---|---|---|---|
| Keyword (BM25) | 6/6, 1.00 | 4/10, 0.27 | 6/6, 1.00 | 4/10, 0.33 | 2/4, 0.40 |
| EmbeddingGemma 2, embedding only | 6/6, 1.00 | 9/10, 0.69 | 6/6, 1.00 | 9/10, 0.73 | 4/4, 1.00 |
| EmbeddingGemma 2, hybrid | 6/6, 1.00 | 7/10, 0.50 | 6/6, 1.00 | 8/10, 0.59 | 4/4, 1.00 |
| EmbeddingGemma 1, embedding only | 6/6, 1.00 | 10/10, 0.68 | 6/6, 1.00 | 10/10, 0.88 | 4/4, 1.00 |
| EmbeddingGemma 1, hybrid | 6/6, 1.00 | 7/10, 0.48 | 6/6, 1.00 | 8/10, 0.71 | 4/4, 1.00 |
| BGE-M3, embedding only | 6/6, 1.00 | 10/10, 0.78 | 6/6, 1.00 | 9/10, 0.79 | 4/4, 1.00 |
| BGE-M3, hybrid | 6/6, 1.00 | 7/10, 0.56 | 6/6, 1.00 | 7/10, 0.61 | 4/4, 1.00 |
| multilingual-e5-small, embedding only | 0/6 | 1/10 | 0/6 | 0/10 | 0/4 |

## Cost (CPU, includes the CUDA-capable server process, so a built-in embedder would use less)

| Model | File | Licence | Dim | Load | PSS after load | PSS after embedding 230 texts | Speed | One query |
|---|---|---|---|---|---|---|---|---|
| EmbeddingGemma 2 (Q8) | 295 MB | Apache-2.0 | 768 | 1.6 s | 767 MB | 1180 MB | 27 texts/s | 21 ms |
| EmbeddingGemma 1 (Q8) | 318 MB | Gemma terms (not Apache) | 768 | 1.0 s | 731 MB | 1030 MB | 37 texts/s | 18 ms |
| BGE-M3 (Q8) | 605 MB | MIT | 1024 | 1.4 s | 1110 MB | 1277 MB | 19 texts/s | 22 ms |
| multilingual-e5-small (Q8) | 125 MB | MIT | 384 | 0.8 s | 523 MB | 781 MB | 100 texts/s | 3 ms |

## What it says

- Keyword search alone finds exact words in both languages (12/12) but fails on meaning: 4 of 10 paraphrases in each language, and 2 of 4 across languages. A small embedding model fixes that: 9 or 10 of 10 in both languages, and 4 of 4 across languages.
- EmbeddingGemma 2, EmbeddingGemma 1 and BGE-M3 are close. On this test EmbeddingGemma 1 and BGE-M3 ranked slightly better than EmbeddingGemma 2, but the difference is one or two questions out of ten.
- The hybrid I tried (equal-weight rank fusion) was worse than embedding only on paraphrases, though still better than keyword alone. Keyword matches on common words push wrong texts into the fused list. This is a tuning problem (weights, a keyword boost only when the word is rare, or using keyword only as a tie-break), not proof that hybrid is bad. It was fixed twice during the run (zero-score documents, stopwords) and is not tuned further, since tuning on 34 questions would overfit.
- multilingual-e5-small scored near zero even on exact words. That points to a bad GGUF or a mismatch with the engine, not to the model itself (the first file would not load at all: `bert model needs to define token type count`; the second one loads but gives poor vectors). Treat e5-small as untested, not as failed.
- Cost is small for all three good models: 300 to 600 MB of file, about 1 to 1.6 s to load, about 20 ms per query on the CPU.

## Recommendation (not a decision)

EmbeddingGemma 2 text-only fits the spec's first choice: Apache-2.0, 295 MB, 768 dimensions, Arabic and English both worked, it loads in under 2 s and answers in about 20 ms. BGE-M3 is the fallback if Arabic quality matters more than size. Semantic search clearly pays off over keyword only. Hybrid needs tuning before it is worth its complexity.

## Limits

- Tiny gold set (10 memories, 34 questions) written by me, including the Arabic. A native Arabic reader should check the Arabic texts and questions. Real Arabic conversation text was not available on this machine.
- The real distractor text is English only, so the Arabic pool is easier (30 fillers).
- The memory numbers come from a llama.cpp server with the CUDA libraries loaded. A built-in embedder (for example through a Rust crate) needs a separate RAM measurement when Milestone 2 starts.
- EmbeddingGemma 2 was tested through the `ggml-org` GGUF, which is a text-only 295 MB file with a separate vision and audio projector file that was not downloaded.
- Models are in `models/` and ignored by git.

## Addendum: smaller quantizations and Nomic Embed Text v2 (same test, CPU only, GPU hidden)

Same 230 texts and 34 questions. Embedding only, top-5 hits (paraphrase EN / paraphrase AR / cross-language):

| Model and file | Size | RAM after load | One query | Paraphrase EN | Paraphrase AR | Cross |
|---|---|---|---|---|---|---|
| EmbeddingGemma 2 Q8 (earlier run) | 295 MB | 767 MB | 21 ms | 9/10 | 9/10 | 4/4 |
| EmbeddingGemma 2 UD-Q6_K_XL | 237 MB | 501 MB | 25 ms | 9/10 | 9/10 | 4/4 |
| EmbeddingGemma 2 UD-Q4_K_XL | 167 MB | 495 MB | 16 ms | 9/10 | 8/10 | 4/4 |
| Nomic Embed Text v2 Q4_K_M | 328 MB | 721 MB | 15 ms | 9/10 | 10/10 | 4/4 |
| Nomic Embed Text v2 Q8_0 | 488 MB | 776 MB | 19 ms | 9/10 | 10/10 | 4/4 |

- The quantization barely matters. EmbeddingGemma 2 lost one Arabic paraphrase at Q4 and nothing at Q6.
- Nomic v2 matched or beat EmbeddingGemma 2 by one Arabic question, at a larger size and more RAM. One question out of ten is inside the noise of this test.
- The hybrid (BM25 plus embedding) was again worse than embedding only on every model. Same note as before: the fusion needs tuning.
- Not run: Needle 3. It needs the Cactus engine and a Python package, not llama.cpp, and its card lists no languages. Installing an outside package for a long shot was not worth it here.
- Same limits as before: tiny test set, Arabic written by the model, native reader needed.

Reading: nothing here forces a change. EmbeddingGemma 2 at Q6 (237 MB) or Q4 (167 MB) is the small option; Nomic v2 Q4 is a close alternative with a better Arabic score. This is a recommendation, not a decision.
