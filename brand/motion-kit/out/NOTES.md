# Motion v1 notes

Generator: `tools/build_v1.py` (re-run from `brand/motion-kit/`). It writes `out/lottie/`. v0 (`lottie/`, `tools/build_mark.py`) is untouched for comparison.

## What changed and why
- **Mark, working loop**: eyes travel on arcs (spatial tangents), the face lean starts 3 to 5 frames after the eyes, the eyes overshoot (1.05) and settle after the face. The slot pop has a secondary bounce (108, 97, 100). The look-back is a real double-take (look, pull back, look again, then back). Asymmetric blinks (3 frames closed, 6 open, far eye 1 frame late). Spawn is a drop with stretch, squash 118 × 84 and a damped settle. No shadow: the brand has no shadow rule, so none was added.
- **Moods** as markers in every mark file: idle, thinking, done, asleep, needs-you, failed. Eye shapes come from `brand-geometry.js`. Every segment starts and ends on the neutral pose (see `timing-sheet.md`).
- **Six bots**, with the identity page's tool actions and its sphere math for the turn. Background solid #1a1b26 in the `-bg` files.
- Loop length: working is 408 frames, the same as v0.

## Exports
- `out/lottie/`: 4 mark palettes and 6 bots, each with and without background (20 files).
- `out/renders/`: working loop of the mark, grove-dark: MP4 (H.264), WebM with alpha (VP9), GIF 256 px (30 fps, loops), APNG 256 px.
- `out/sprites/`: `tray-32-<mood>.png` (6 frames each, rendered from the Lottie at 32 px) and `tray-16-<status>.png/.svg` (the hand-placed pixel mark from `brand-geometry.js`, 2 frames: slot lit, slot dimmed) for idle, busy, ready, needs-you.
- `out/qa/`: a contact sheet for every file (every 12th frame).

## Checks (`tools/qa.sh`)
- Run: `NODE_DIR=<dir with render_v1.js as render.js, playwright-core, lottie-web> CHROME=… tools/qa.sh`.
- Every segment except spawn has a pixel-identical first and last frame in lottie-web. That covers every loop, and every clean cut. Spawn starts from nothing, so it is skipped. Result: all pass.
- The contact sheets were looked at. Eyes stay on the face and the slot keeps its gap in every sheet.

## Not done, and why
- **Bounds check as a script** (safe circle r30, eye and slot limits): only checked by eye on the contact sheets, not by a script.
- **Second player** (rlottie, velato): not tried. Neither is installed here, and this run did no installs or builds while another agent held the machine lock. Not verified beyond lottie-web.
- **dotLottie** bundle with palettes as themes: not made. The tooling was not checked, so nothing is faked.
- **Bot video and GIF renders**: not made, for the same lock reason. The Lottie files are there; run the same ffmpeg steps on them.
- **Social video** (optional): not made.

## Still to improve
- Bot files are about 215 KB each, because the 3D turn is baked every 2 frames. Baking every 3 or 4 frames would halve that.
- The bot eye rotation and highlight drift are approximations of the page's maths; the turn could use more ease on the far eye's hide.
- The 16 px tray sprite only changes the slot. It has no moods, because the pixel mark has one hand-placed pose.
