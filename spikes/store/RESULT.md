# Spike 4: local store

Question: can SQLite be the store for memory (pages, keyword search, vectors, append-only history, owner and scope on every row)?

How: `bench.py` (Python 3 with numpy, SQLite 3.53.4 with FTS5) builds a database with the planned shape and measures it at 1k, 10k and 100k pages of 25 to 60 words, half English and half Arabic. Schema: `pages` (owner, scope, layer, privacy, archived flag), an FTS5 index over `pages` (unicode61 tokenizer, diacritics removed), `embeddings` (768 floats as a blob), and an append-only `history` table filled by triggers and protected by triggers that refuse update and delete. Write-ahead log (WAL) mode is on.

| Pages | Insert pages | Insert vectors | File size | Keyword query | Load all vectors | Vector scan (top 10) |
|---|---|---|---|---|---|---|
| 1,000 | 0.03 s | 0.03 s | 4.9 MB | 0.2 ms | 3 MB in RAM | 0.7 ms |
| 10,000 | 0.39 s | 0.23 s | 48 MB | 2.3 ms | 29 MB, 0.02 s | 7 ms |
| 100,000 | 4.4 s | 1.9 s | 482 MB | 25 ms | 293 MB, 0.3 s | 70 ms |

Other checks, all passed:
- An edited page is found by the keyword index at once, and the old text is kept in `history` with the new text.
- Deleting from `history` is refused (append-only holds).
- Arabic words match in FTS5 with the plain tokenizer (exact word forms).
- WAL: a writer committing about 470 times a second did not slow a reader (median 2.7 ms, worst 4.1 ms over 1,822 reads).
- `kill -9` on a writer in the middle of a write: `integrity_check` is ok, the FTS index check passes, and the page count matches the history count.

Answer: SQLite is enough. A personal memory is far below 100,000 pages, and at that size a plain scan over vectors held in RAM takes 70 ms and uses 293 MB. Under 10,000 pages (the likely range) it takes 7 ms and 29 MB. No vector extension or separate vector database is needed for the first release. Scaling past roughly 100,000 pages would be the time to look at a vector index, or at compressed vectors.

Recommendation (not a decision): keep SQLite as the store, with FTS5 for keywords and vectors stored as blobs that are loaded on demand and dropped in idle mode, which fits the spec.

Limits:
- Random word text, so keyword queries match nearly every row. That makes the keyword times a worst case.
- Python's SQLite, not the Rust `rusqlite` crate. Both use the same C engine, but build flags (FTS5 enabled, bundled or system library) must be checked when the crate is added in Milestone 1.
- No Arabic stemming in FTS5. Words with prefixes (for example `والإيجار` against `الإيجار`) do not match each other. Spike 3 handled this with light prefix stripping outside the database. If keyword search matters for Arabic, the stripping needs to happen before indexing.
- Encryption at rest, and sync, were not tested.
- The multi-user design (many owners) was not load tested. Every row carries `owner` and `scope`, which costs almost nothing.
