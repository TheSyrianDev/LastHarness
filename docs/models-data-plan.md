# Models page data layer (plan, no UI)

Status: plan only. No code yet. The CEO session holds the go-ahead for code and for `cargo` runs.

## What we read

- `~/Projects/hf-dl/docs/EMBEDDING.md` (current): `Config::from_env`, `auth::resolve_token`, `HubClient::new`, `list_tree` (path and size), `parse_quant(path)` returning name and bits per weight, `parse_shard(path)` for split GGUF files, `Downloader` with `download_file_cancellable`, `DownloadOptions` (cache_dir, local_dir, engine, connections, max_speed, verify, force), `EventSink` events (FileStart, Progress, FileDone, Message), `CancelToken`, `.hfd.state` resume.
- The doc does not yet show what the CEO described: Hub search with GGUF filter, files grouped by quant with sizes, shards merged, mmproj detection, a fits/tight/too-big helper, and a progress stream. Those are in progress in hf-dl. We plan against that API and do not build our own Hub client.
- Download log: `HFD_DOWNLOAD_LOG` (JSON entries: repo, file, quant, size, path, sha256).
- Registry: `~/AI/Rig/registry/models.json`. Top-level keys: `schema`, `generated`, `limits`, `models`. Each model has `id`, `repo`, `file`, `quant`, `size_bytes`, `kind`, `ctx_max`, `mmproj`, `preferred_engine`, `llama_swap_entries`, `engines`. Some entries have `repo: null` (local files only).

## Data the page needs

1. Search results: repo id, name, downloads, GGUF present (yes or no).
2. Per repo: files grouped by quant, with size in bytes, shard count (merged), mmproj file if any.
3. Fit per quant: `fits`, `tight` or `too-big`, from free RAM and VRAM (the helper comes from hfd-core; we pass the figures).
4. Download state: queued, running (bytes, speed, ETA from the progress stream), done, failed, cancelled.
5. Installed models: from the registry (`models` list), so the page can mark what is already on disk.

## Where it goes in our code

- New module in `crates/lh-core` (for example `models_catalog.rs`) with pure data types and functions. Tests run without the network.
- Thin adapter over hfd-core behind one trait (`CatalogSource`), so tests use a fake and the real crate is used only at runtime.
- Store download jobs in the core, emit progress as SSE events (existing event stream), and write the registry entry when a download completes.
- API routes later, under `/v1/models/catalog/...`. No GUI in this step.

## Licence

hfd-core is Apache-2.0 (per EMBEDDING.md) and can be bundled. The `hfd` CLI is PolyForm Noncommercial. We depend on the library only. Add a line to THIRD-PARTY-NOTICES.md when the dependency lands.

## Open questions (for the CEO, then Omar)

1. Does the hfd-core Hub search and fit helper have a stable version to pin, or do we take a git rev?
2. Is the download log written by hfd-core or by the caller? Our plan assumes the library writes it when `HFD_DOWNLOAD_LOG` is set.
3. Where do downloaded files live: the HF cache (shared with the `hf` CLI) or `~/AI/Models`? Default proposal: `local_dir` under `~/AI/Models/<name>/`, so the registry paths stay simple.

## Steps (after the go)

1. Pin a hfd-core revision once the search and grouping API lands; until then, write the types and the fake source.
2. Core module and tests (no network).
3. Adapter over hfd-core and a test that uses only the public types.
4. Registry write on completion.
5. Report to the CEO session. No UI.
