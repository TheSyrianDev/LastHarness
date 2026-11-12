# Models data layer: handoff

Status: not compiled, not tested, not committed.

## Done
- `crates/lh-core/src/models_catalog.rs` (new, uncommitted): types, `group_by_quant`, `mmproj_files`, `fit_for` (registry rule: fits if size <= free RAM, tight up to 10% over, else too big; `free_vram` kept for the GPU offload hint), `parse_shard`, `mmproj`, `CatalogSource` trait, `FakeCatalog`, tests.
- `crates/lh-core/src/lib.rs`: `pub mod models_catalog;` (uncommitted).
- Plan: `docs/models-data-plan.md` (uncommitted).

## Next
1. Wait for the CEO session's go (lastengine-74 holds `~/AI/.machine-busy`).
2. Compile and test the data layer: `cargo check -p lh-core -j2`, then `cargo test -p lh-core`, under the lock.
3. Wire in hfd-core (path dep behind a cargo feature) using the names in `~/Projects/hf-dl/docs/EMBEDDING.md` at commit 36284c1. Ask Omar before any git rev pin.
4. Replace `fit_for` with hfd-core's helper once it lands; keep one rule.
5. Commit only after build and tests pass, as the user, own files only. Do not push.
