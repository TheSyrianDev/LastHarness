# LastHarness motion kit

A brief, source files and a first real animation, to hand to Claude Code.

1. Unzip into a new folder (for example `~/lastharness-motion`).
2. Open a terminal there and start Claude Code (`claude`).
3. Say: **"Read PROMPT.md and do it."**

It will need Node, Python 3, ffmpeg and Chromium (Playwright) — it can install what's missing.

To watch the current animation yourself: run `python3 -m http.server` in this folder and open http://localhost:8000/viewer/ — or just play the files in `renders/`.

Contents: `PROMPT.md` (the brief) · `timing-sheet.md` (the spec) · `source/` (rigged SVG, tokens, brand geometry, bot tools) · `lottie/` (v0 animation, 4 palettes) · `renders/` (MP4, WebM, GIF) · `tools/` (generator and renderer) · `viewer/` (segment player) · `reference/` (key frames and the identity page).
