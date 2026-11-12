# Changelog

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
