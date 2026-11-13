# Changelog

## Unreleased

- Renamed from LastHarness to Last AI. Config and data move to `~/.config/lastai` and `~/.local/share/lastai`; the old `lastharness` folders are copied over on first run. `LASTAI_*` env vars are read first; `LASTHARNESS_*` still work. Binaries: `lastai-gui`, `lastaid`, `la`, `lastai`.

## 0.1.0-alpha

First alpha. Built from the code in this repository; see the sections above for what each part does.

### Added
- Local core `lhd` with an HTTP API and SSE, a terminal app (`la`, alias `lh`) and the desktop app (`lastai-gui`).
- Models and keys: search GGUF models by fit, download through hfd-core, run and use installed models.
- Create: image and video jobs through a local ComfyUI.
- Team: departments and workers; outside CLI workers bound in `~/.lastharness/external-workers.json`, writes need approval.
- Chat: `/usage`, `/insights`, `/rollback`, `/budget` in terminal and desktop; `@git diff` references.
- Memory undo, snapshots and rollback, toolsets, schedules including `every weekday at 9`.
- Local add-on hub (folder version) in `lh-core`.
- Headless smoke script `scripts/smoke-headless.sh`.

### Changed
- Compaction keeps the system prompt, the first user message and the last turns verbatim.
- Qwen-style raw `<function=...>` tool calls in replies are recovered as tool calls.

### Known gaps
- `@URL` references are listed but not fetched in chat.
- The Mercury live checklist is not built.
- The add-on hub has no UI or routes.
- Parallel subagents do not yet use the file-locked edit transaction.
- Wan 2.2 colour noise was traced to a 64-bit seed; the live check after the fix is pending.

## Unreleased

### Added
- Models and keys page: search Hugging Face GGUF models grouped by fit (Fits in memory, Tight, Too big, Size not known), with downloads through hfd-core.
- Installed model list with Run (registers an unserved GGUF with llama-swap, with backup and matrix validation) and "Use in chat".
- Create: image (Z-Image Turbo) and video (Wan 2.2 TI2V-5B, MP4) jobs through a local ComfyUI, with a job list and inline results. Jobs take the machine lock and free GPU memory afterwards.
- Web search finds a SearXNG on `127.0.0.1:8888` when none is configured, and says so when nothing answers.
- Debug hooks for screenshots and tests: `LASTHARNESS_VIEW`, `LASTHARNESS_MODELS_QUERY`, `LASTHARNESS_CHAT_TEST`.

### Known issues
- Wan 2.2 video output showed colour noise in early live runs. The workflow matches the reference; the cause is not yet confirmed.
- The Create block shows results but does not play videos inside the app; it shows the first frame.
- Download progress is shown only in the job list, not live in the page.
