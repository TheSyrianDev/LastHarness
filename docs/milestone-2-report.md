# Milestone 2 report: memory v1

Written by the build session while the user was away.

## What exists

New crate `lh-memory`, plus memory in `lh-store`, `lh-tools`, `lh-core` and `lh-tui`.

- **Four layers.** Preferences, facts about you, and projects and decisions are pages: one Markdown file each, with a header (id, layer, privacy, pinned, source, version, dates, a one-line gist), `[[links]]` between pages and a generated `index.md` per layer. Past conversations are rows in the database, stored as they were said and never edited.
- **History.** Every change to a page is recorded in an append-only table that the database refuses to edit or delete. The only allowed change blanks the text of a page the user deleted for good. When a fact changes, the old version is kept in `_archive/` and the page is archived, not overwritten: left out of normal recall, still searchable, restorable.
- **Memory as files.** Files edited by hand are picked up on the next scan: edits become a new version (old one archived, logged as a hand edit), a new file gets an id, a removed file archives its page. Export writes the folders, the archive and the conversations as Markdown; import reads them back and skips pages already present.
- **Search.** Keywords always work (SQLite full-text search with English word forms and Arabic folding). Meaning-based search uses EmbeddingGemma 2 through a local llama.cpp server that starts on first use and stops when the core goes idle, or any embedding endpoint (a non-local one is refused unless allowed, and never receives sensitive pages). Results are fused with a weighted rank (keyword 0.35, meaning 1.0).
- **Bounded requests.** At most 6 notes, 600 characters each, 3,000 in all, plus an "about you" block of at most 1,200 characters built from one-line gists (pins first, then preferences, facts, projects), never written by a model. A note already in the conversation is not added twice, so the start of every prompt stays identical for the server's cache. The `recall` tool returns a short list and `open_note` reads one whole note.
- **Privacy.** Patterns catch credentials, health, finance, identity documents, a `#private` marker, and unsure identifiers (emails, phone numbers, long tokens). On a local model the model also judges what the patterns pass; unsure or unreachable counts as sensitive. One rule, `visible`, decides what may be sent, and every path that builds model text goes through it before ranking. Sensitive notes go only to a destination with trust level Local.
- **What was sent.** Each request that carries notes writes an activity-log entry naming the pages and versions and the destination's trust level. `lh memory sent` lists them.
- **Review and remember.** The end-of-session review is one more user turn on the existing conversation (same tools listed, so a caching server reads the history from cache). It produces suggestions to add, change or archive notes. Nothing is saved until the user accepts, edits or drops each one (terminal `/review` screen, `lh memory accept|reject`, or the API). The assistant has `remember` and `forget` tools that ask first, and `recall`.
- **Incognito.** No memory is read or written, no conversation is stored, no memory tools, review refused.
- **Maintenance.** Finds duplicates (suggests archiving the older page), broken links, stale facts, unlinked pages, and with a local model possible contradictions. It changes nothing by itself; it can run when the core goes idle.
- **Viewer.** `lh memory list|search|show|add|archive|restore|delete|pin|sensitive|normal|scan|proposals|accept|reject|maintenance|sent|conversations|export|import`.

## How it was checked

148 automated tests pass across the workspace and `cargo clippy` is clean.

- **Exit rule, as a test.** 120 combinations of pages and questions with a cloud destination: no sensitive text, note id, or gist reached the request, including pinned pages, pages marked sensitive by hand-editing the file, and meaning-search hits. The same questions to a local destination do find them, so the test is not empty. The same check runs through the HTTP API.
- **Real exit run** (a 35B coding model as the local model, real EmbeddingGemma 2): the user said they were Omar, lived in Berlin, liked short answers, and gave a wifi password. The review took 13 seconds and suggested three notes (nothing was saved before accepting). After accepting, the core was restarted: a fresh session answered "You live in Berlin." The wifi password was never suggested for saving, and the model said it did not have it. With the same files and the endpoint treated as a cloud destination, only the one ordinary note was sent; the two sensitive pages were not.
- **Real embedding quality.** With EmbeddingGemma 2 on 8 paraphrase questions in each language, keywords alone found 3/8 English and 5/8 Arabic in the top 5; keyword plus meaning found 8/8 in both. Starting the embedding server and embedding 26 pages took 2.7 seconds.

## What is not done

See `.grill/parked.md`, section Milestone 2: the privacy defaults are cautious (an unsure local model flagged "Lives in Berlin" as sensitive); only two privacy classes; embedding model not bundled; editing a suggestion's text inside the terminal; the Arabic test texts need a native reader.
