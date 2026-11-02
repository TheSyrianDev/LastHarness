# Brief for Claude Code — LastHarness motion

You are the motion designer and motion engineer for **LastHarness**, a local-first AI harness (Rust core, command `lh`, Linux first, Omarchy desktop). The logo and the six worker bots are finished as static designs. Your job is to turn them into **real, polished, keyframed animation files** the apps can play, plus renders for the web and social. Work in this folder. Take your time; quality of motion matters more than speed.

## What is already here
- `source/mark-rig.svg` — the mark split into named, pivoted parts (face, eyes, eye-near, eye-far, slot, slot-ring). Parenting and pivots are in the comments and `data-pivot` attributes.
- `source/tokens.json` — exact geometry, palettes, status colours, bot colours, forbidden uses.
- `source/brand-geometry.js` — the code that draws the static mark, moods, wordmark and lockups (reference for shapes).
- `source/bot-tools.js` — the six tools drawn by each worker (compass, magnifier, pen, terminal, brush, clipboard).
- `timing-sheet.md` — the "appear and glance" animation, beat by beat, with frames and easings. **This is the spec.**
- `lottie/` — **v0**, a first keyframed Lottie of the mark that follows the timing sheet (8 files: 4 palettes, with and without background). It works, but it is a first pass.
- `tools/build_mark.py` — generates `lottie/` from the timing sheet. `tools/render.js` — renders any Lottie to PNG frames with lottie-web in headless Chromium.
- `renders/` — MP4, WebM (transparent), GIFs rendered from v0.
- `reference/lottie-v0/` — key frames of v0. `reference/web-prototype/` — frames of the earlier JS prototype the user liked. `reference/identity-page-v1.html` — the full brand page (open it in a browser to see the moods, bots and their motion).
- `viewer/index.html` — plays every Lottie with buttons for each marker segment, a scrubber and 0.25× speed. Serve the folder (`python3 -m http.server` at the root) and open `/viewer/`.

## Ground rules
1. **Do not redesign.** Shapes, proportions, colours and the glance (translate 2.4, −2; rotate −8° about (27,37)) are locked. You are animating, not designing.
2. **The source of truth is code you can re-run.** Keep a generator (extend `build_mark.py`, or replace it with a better authoring path such as the `lottie` Python package, if you can justify it). Never hand-edit the exported JSON.
3. **Look at your work.** After every change, render frames, build a contact sheet, and actually view the images. Scrub the slow parts at 0.25×. Judge it like an animator, not like a test suite.
4. **Keep `timing-sheet.md` in sync** with what the files actually do.
5. Keep files small and player-friendly: shape layers only, no expressions, no effects, no images, no text layers, no precomps unless needed. Must play in lottie-web, and must not use features `rlottie` and the Android/iOS players lack.
6. Check before you claim. If you say a Rust player works, you ran it.

## Phase 1 — Polish the mark (the most important part)
Make v0 feel alive and crafted. In particular:
- **Arcs:** the eyes should travel on a slight arc during the glance and the look-back, not in a straight line.
- **Overlapping action and follow-through:** the face leans a few frames *after* the eyes start; the eyes settle a few frames after the face; the slot's pop has a small secondary bounce.
- **Anticipation and timing:** the anticipation before the glance should read but stay subtle. The "looking for it" beat after the slot vanishes should feel like a real double-take.
- **Blinks:** asymmetric (close fast, open slower); the far eye may lag the near eye by one frame.
- **Spawn:** a satisfying landing, with squash on impact and a clean settle. Consider a tiny shadow/contact cue only if it stays inside the brand rules.
- **Loop seam:** frame 84 and the last frame must match exactly.
Keep the total loop length within ±20% of the current one unless the motion clearly improves.

## Phase 2 — The moods, as segments
Add named markers (one file per palette, all segments in it) for the moods on the identity page: **idle** (looking at you, breathing), **working** (the glance loop above), **thinking** (eyes up, moving between you and the slot, slot pulsing in the busy colour), **done** (happy eyes, a small hop, slot turns ready-mint), **idle mode / asleep** (eyes closed lines, slow breathing, slot dimmed), **needs you** (eyes wide, on you, slot in needs-you clay, a gentle attention bounce), **failed** (x eyes, a small shake). Each segment must start and end in a pose the others can cut to cleanly (document the "neutral" pose). Status colour changes on the slot should be animated, not cuts.

## Phase 3 — The six worker bots
Make one Lottie per worker (Lead, Researcher, Writer, Coder, Designer, Planner; colours and tools in `tokens.json` and `bot-tools.js`; background #1a1b26). Each bot is a **ball** (no ears) with two pill eyes, and its tool sits up and to the right, where the logo's slot is. Segment **work**: the tool starts its action first (compass needle spins; magnifier sweeps; pen scribbles; terminal cursor types with the window bobbing; brush dabs; clipboard tick draws itself), then the bot **turns its whole body and looks up at the tool in one move** (about 56° turn and 38° up). Fake the 3D turn convincingly: the eyes travel across the sphere and foreshorten, the far eye slides behind the edge, and a soft highlight and a faint seam line shift to sell the rotation. Then it turns back and blinks. Also give each bot **idle** and **spawn** segments matching the mark's.

## Phase 4 — Exports
For the mark and every bot, export:
- `.json` Lottie per palette, and one **dotLottie** (`.lottie`) bundle with the palettes as themes if your tooling supports it (check; do not fake it).
- MP4 (H.264), WebM with alpha, GIF (256 px, seamless loop), and APNG for the mark.
- A **tray sprite**: the mark at 16 and 32 px, a few frames of each mood, as PNG sprite sheets (pixel-snapped; at 16 px use the hand-placed pixel version from `brand-geometry.js` as the base).
- A 1200 × 630 social video (mark appears, glances, lockup "lastharness" slides in) — optional, last.

## Phase 5 — Checks (write them as a script and run it)
- Loop seam: first and last frame of every looping segment are pixel-identical.
- Bounds: outside the r30 safe circle there are no pixels, except during the ring burst and the spawn overshoot.
- Eyes never leave the face; the slot never comes closer than 3 units to the face.
- Every file loads and plays in lottie-web; try a second player too (e.g. `rlottie` via Python bindings, or the `velato` Rust crate if it exists and builds — verify, and report honestly if a player can't render something).
- Contact sheets for every segment in `qa/`. Look at them.

## Deliver
Put everything in `out/` (`out/lottie`, `out/renders`, `out/sprites`, `out/qa`), update `viewer/` so it lists every file and segment, update `timing-sheet.md`, and write `out/NOTES.md`: what you changed and why, what you'd still improve, and anything you could not do. Ask me only if you are truly blocked; otherwise make the call and note it.
