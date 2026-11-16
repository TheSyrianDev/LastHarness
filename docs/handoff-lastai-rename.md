# Handoff: rename LastHarness to Last AI (stopped at inventory)

Status: NOT STARTED in code. No rename commits made. Working tree clean before this file.
Stopped at step 1 (inventory) because of the wind-down message. Nothing unfinished is committed.

## Decided (owner, chat m04200 "Yes.")
- User-visible "LastHarness" becomes "Last AI" (text), "LastAI" (repo, identifiers, file names), "la" (command).
- Config/data dirs and `LASTHARNESS_*` env vars move to the new names. Old names still read as fallback. First run migrates files (copy, not move).
- Internal `lh-*` crate names stay.
- Why: `lh-` now reads as *harness component*, the thing these crates are (core, client, tui, gui, team, tools, memory, provider, spend, secrets, store, learn, mcp, channels), not as an abbreviation of the old product name. The harness is the layer Last AI, LastEngine and hf-dl sit around, so the prefix is the accurate one to keep. This is settled: do not propose renaming them again.

## Inventory (`git grep -i lastharness`, excluding target and third_party)
- 299 files total. Roughly 60 outside research/, docs/archive/, .grill/.
- Code with path or env names: crates/lh-config/src/lib.rs:183, crates/lh-core/src/{auth.rs,main.rs,team_api.rs,provider_api.rs,mcp_api.rs,export_api.rs,memory_notes.rs,create_api.rs}, crates/lh-tui/src/{terminal.rs,setup.rs,service_cli.rs,team_cli.rs}, crates/lh-gui/src/{config.rs,prefs.rs,fonts.rs,theme.rs,app.rs,ui.rs,tray.rs,pages/service.rs}, crates/lh-tools/src/{perm.rs,snapshot.rs}, crates/lh-mcp/src/lib.rs (client name), crates/lh-tools/src/web.rs (user agent).
- Binaries: crates/lh-tui/Cargo.toml [[bin]] `lastharness` (src/bin/lastharness.rs); `lastai` bin already exists (src/bin/lastai.rs). crates/lh-gui/Cargo.toml package/bin `lastharness-gui`.
- Packaging: packaging/lastharness.desktop, packaging/lastharness.png, packaging/build-appimage.sh, packaging/aur/PKGBUILD, packaging/omarchy/{README.md,lastharness-icon-refresh,theme-set-hook}, scripts/install-omarchy.sh, .github/workflows/release.yml (artifact names, .app bundle, dmg, zip).
- Service unit: `lastharness.service` (crates/lh-gui/src/pages/service.rs, crates/lh-tui/src/service_cli.rs).
- GUI/UI text: crates/lh-core/web/{index.html,app.js,style.css}, crates/lh-gui/src/ui.rs (actions! namespace, key_context), themes/*.toml headers.
- Tests and debug env: crates/lh-gui/tests/gui_drive.py, gui_resume.py (run `target/release/lastharness-gui`; do not run them, no GUI).
- Docs: README*.md (5 languages), AGENTS.md, CHANGELOG.md, COMMERCIAL-LICENSING.md (keep contact line), docs/spec, docs/digest.md, docs/roadmap.md, docs/*.md. Historical: docs/milestone-*-report.md, docs/handoff-*, docs/archive/ keep old name, add "formerly LastHarness" first line where missing.
- Brand kit: brand/motion-kit/* (lottie file names, tools). Decide before touching: kit is design source, low priority.
- research/ and .grill/: reference or decision log. Leave as is; report as leftovers.

## Plan still to do (in order)
1. Commit (a) user-visible text and packaging.
2. Commit (b) binary and GUI names (`la`, `lastaid`, `lastai-gui`; check Cargo.toml [[bin]] first).
3. Commit (c) config/data migration with tests. Tests: new dir used, old dir fallback read, migration copies, env var fallback. Temp dirs only. Do not touch ~/.lastharness on this machine.
4. Commit (d) docs, historical headers.
5. `cargo test --workspace` under `~/AI/Rig/bin/machine-lock take <name>`, release after.
6. Grep for leftovers outside historical docs (must be zero).
7. Fresh copy: /home/omar/Projects/public/LastAI-fresh2 via `scripts/sync-public.sh --apply`. Author must be TheSyrianDev <215098066+TheSyrianDev@users.noreply.github.com>, one commit. Scan for /home/omar, Baroud, omarbarood, @gmail, /run/media/omar, token patterns. Do not touch LastAI-fresh or LastAI public copies.

## Open
- Owner email question still open: COMMERCIAL-LICENSING.md contact line must not change.
- Dev repo is now at /home/omar/Projects/LastAI. The old path /home/omar/Projects/LastHarness no longer exists.
- No push, no GitHub changes until the owner says so.
- Note: a fresh copy made now would still carry the LastHarness names, so step 7 waits for the rename.

## Status (2026-11-13)

- Done: config/data/token paths with old-name fallback and migration (b7f5ded, tests in lh-config appdir.rs); user-visible text, packaging and release names (f978f8c).
- Not done: docs/ (outside archive), README.fr/de/ar/zh-CN, CHANGELOG, brand assets (lastharness-lockup-*.svg), historical "formerly LastHarness" headers, sync-public.sh default path, lh-pageview-engine test fixture.
- Kept on purpose: keychain service "lastharness", systemd unit lastharness.service, project .lastharness/ folder, export/memory format read compat, machine lock owner.
