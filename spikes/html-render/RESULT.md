# HTML pages inside the side panel: Blitz, litehtml and Servo

Throwaway spike, 2026-10-12. Question: how can the desktop app draw a local HTML page inside the side panel, without a Chromium web view, and without scripts running by default? Nothing here opened a window or touched the mouse: every page was drawn to a PNG file in memory and the PNGs were looked at. Nothing was sent anywhere except fetching crates and reading the three projects' README pages.

## Short answer

Use **Blitz** as the default viewer for web pages without JavaScript. It draws grid, flex, shadows, gradients and right-to-left Arabic correctly, repaints a scrolled frame in 2 to 5 ms, and every request a page makes goes through one function the app writes, so it can refuse all of them. Keep **Servo** as the optional add-on for pages that need JavaScript. **litehtml** is not worth using: it has no grid and it drew no Arabic letters at all.

Blitz is beta and its crates move fast (two of its own crates already disagree on a dependency version, see "Risks"), so pin exact versions and keep the viewer behind one small module.

## What was checked, and how

Pages (in `pages/`): `index.html` (the page the model wrote in the user's home folder), `article.html` (CSS article with table, quote, gradient box with shadow), `arabic.html` (Arabic and mixed right-to-left text, list, table, input), `layout.html` (inline CSS grid, flex, flex-wrap, links), `js.html` (inline JavaScript, promises, canvas), `net.html` (asks for remote CSS, a script, remote and local images, a frame of `/etc/hostname`, a background from the web).

Programs: `blitz/` (about 120 lines on `blitz-html`, `blitz-dom`, `blitz-paint`, `anyrender_vello_cpu`) and `litehtml/` (about 60 lines on the `litehtml` crate with its `pixbuf` backend). Run: `hr-blitz PAGE OUT.png [WIDTH]`, `hr-blitz --interact PAGE PREFIX`, `hr-litehtml PAGE OUT.png [WIDTH]`. Pictures are in `out/`.

Candidates verified at their upstream pages:

| | Blitz | litehtml (Rust crate `litehtml`) | Servo |
|---|---|---|---|
| Upstream | github.com/DioxusLabs/blitz; crates on crates.io 0.3.0-beta.2 | engine github.com/litehtml/litehtml (C++); Rust bindings github.com/franzos/litehtml-rs, crate 0.2.6, 28 commits, one author | servo 0.7.0 (tested on 2026-10-11, `spikes/gui-toolkits`) |
| Self-described status | "beta"; renders "many popular no-JS websites"; "many bugs and missing features" | "not fully compatible with HTML/CSS standards", "not recommended as a full-featured HTML engine"; bindings say "modern JS-heavy sites with megabytes of CSS will be slow or render incorrectly" | pre-1.0, breaking changes expected monthly |
| JavaScript | none (no engine; `<script>` is never run or fetched) | none (the bindings' `html` feature strips scripts and frames) | yes (SpiderMonkey) |
| Licence | MIT OR Apache-2.0; its CSS engine Stylo and 13 related crates are MPL-2.0 (file-level copyleft, fine to link); ICU data under Unicode-3.0 | engine BSD-3-Clause, gumbo parser Apache-2.0, bindings MIT; one dependency is `Apache-2.0 OR GPL-2.0-only` (we would take Apache) | MPL-2.0 |
| Native build needs | Rust only (1.89 or newer, edition 2024) | a C++17 compiler (g++ here) | Rust only, large |
| Native libraries linked | fontconfig, freetype, expat, libpng, bz2, brotli, zlib (same family the app already uses for fonts) | libstdc++ | fontconfig, freetype, expat, libpng, bz2, brotli, zlib |
| Crates in the build | 267 | 75 | many more |

## Numbers (this machine, release build, 800 px wide page, CPU rendering)

| | Blitz | litehtml |
|---|---|---|
| First build (12 cores, `nice 19`, from empty) | 139 s, then 36 s for a change in our code | 152 s |
| Binary of the test program | 23.3 MB, 17.7 MB stripped | 6.8 MB, 5.7 MB stripped |
| For comparison | the desktop app is 63.9 MiB (46.2 stripped), so Blitz adds about 14 to 18 MB if linked in | adds about 5 to 6 MB |
| Parse, style and layout, one page | 17 to 32 ms | 11 to 17 ms (the very first page 341 ms: fonts loading) |
| Draw to a picture, whole page | 5 to 10 ms | 20 to 60 ms |
| Repaint of a scrolled 800x500 frame | 1.8 to 5 ms | not measured (it redraws the page the same way) |
| Memory of the whole test process with one page | 20 to 26 MB | 10 to 20 MB |
| Servo for comparison | 110 to 150 MiB more binary, 130 to 160 MB per open page, a page is shown as a picture with up to half a second delay (earlier spike) | |

Blitz memory is the process total, including the system font index, so a second page in the same app costs less than the first.

## Quality, page by page (see the PNGs)

| Page | Blitz | litehtml | Servo (earlier spike) |
|---|---|---|---|
| `article.html` | Very close to a browser: serif body font, table, quote, gradient box with a soft shadow. | Good, but a sans font replaced the serif, and the box has no shadow. | Correct. |
| `layout.html` (grid) | Correct: three columns, a card spanning two columns, flex 1:2:1 pills, wrapping tags, rounded corners, gradient. | **Grid is not supported**: all the cards stack in one column. Flex and wrap work. | Correct. |
| `arabic.html` | Correct right-to-left layout, connected letters shaped properly, the mixed line (Arabic, English, digits) in the right order, list, table and input field mirrored. One small fault: list bullets sit at the far left while the text is on the right. | **No Arabic letters were drawn at all** (blank lines and cells; only the Arabic digits showed), although the Arabic fonts are installed. English parts are fine. | Layout right, but some words lost their first letter or had marks misplaced. |
| `js.html` | Draws the static page; scripts do not run ("starting" stays, the canvas stays empty). This is the expected, safe behaviour. | Same. | Everything worked. |
| `index.html` (the user's file) | Blank, and that is correct: the model's file stops in the middle of a CSS rule and has no body at all. It was cut off when written. A browser shows a blank page too. | Blank, same reason. | not tested |

## Links, scrolling, selection

Blitz (tested with `--interact`): scrolling moves the page by whole pixels and a repaint of the visible frame takes 2 to 5 ms, so smooth wheel scrolling is easy. Pointing at a frame position gives the element under it; the app adds the scroll offset to the frame position first (checked: the link at the bottom of the article was found at the right place only after adding it). A click on a link is not acted on by the engine: it calls a `NavigationProvider` that the app supplies with the full address, and a link that only points inside the page (`#end`) scrolls in the page. So the app decides: ask the user, open in the browser, or refuse. Text selection and form controls exist in the engine (`blitz-dom` has selection and form handling); not exercised here.

litehtml: the container reports anchor clicks (`take_anchor_click`) and the app draws at any offset, so scrolling and links are possible; same idea, less behind it.

## Security: one approved file, nothing else

This was the most important check, and `net.html` tested it.

* **Blitz:** the engine never fetches anything itself. Every sub-resource goes to a `NetProvider` the app gives it. The test provider refused all and logged all. For `net.html` it was asked for: the remote stylesheet, `file:///etc/hostname` as a stylesheet, the remote image, a local image outside the folder (`file:///usr/share/pixmaps/...`), a sibling file next to the page, the frame of `/etc/hostname`, and the background from the web. **Seven requests, all refused, the rest of the page drew normally.** The `<script src>` was never requested at all. Two details for the real build: the provider must answer a refused request with empty data (answering nothing leaves the page waiting for its stylesheet and nothing is painted; this was the first result, a blank page), and the app should build the allow-list itself (for example, allow the one approved file and nothing else; or allow images only as `data:` addresses inside the file).
* **litehtml (pixbuf):** the container never loads anything unless the app does. It listed the four images the page asked for (remote, local, sibling, background) and loaded none. Stylesheets, frames and scripts are not fetched by this backend either. Equally safe in design, but see the quality column.
* **Servo:** can be locked down but is a whole browser: it has its own network stack and file access, so it must run in a helper process that is told what to load (and the earlier spike found the helper is not killed when the app dies unless the app arranges that).

## How it would sit in the GPUI panel

Blitz is a library. The panel would hold one parsed page per opened HTML item, draw it with `anyrender_vello_cpu` into an RGBA buffer the size of the panel, and show that as an image (the same `RenderImage` route the painted sky already uses). Scrolling, resizing, hover and clicks map to `scroll_viewport_by`, `set_viewport` and `hit`; each repaint is a few milliseconds, so there is no need for a helper process or PNG files. CSS animations would need a repaint timer. The "Open in your browser" button stays as it is.

## Risks

* **Beta and churn.** `blitz-paint 0.3.0-beta.2` needs `anyrender 0.13` while the newest `anyrender_vello_cpu 0.18` needs `anyrender 0.14`, so a plain "latest" build failed to compile (the first try here). Pin: `blitz-* =0.3.0-beta.2`, `anyrender 0.13`, `anyrender_vello_cpu 0.17`. Expect work at each upgrade.
* **A parser or layout crash would take the app down** if run in-process. Mitigation: run it on its own thread under `catch_unwind`, or in a small helper process later (the 4 ms repaint would then need shared memory).
* **No JavaScript at all** in Blitz; pages that build themselves with scripts show empty.
* **Fonts:** a page asking for Georgia or Arial gets the machine's serif or sans, so text can look different from a browser.
* **Not tested here:** inside the real GPUI window, form controls, text selection, CSS animations, very large pages (the picture is limited to 65,535 px by the CPU renderer; the test clamps at 6,000), images loaded through an allow-list.

## Recommendation (not a decision)

1. Default: Blitz, no JavaScript, network and file access refused except for what the app allows; pages drawn into the side panel as an image of the visible part.
2. Optional add-on: Servo as a helper for pages that need JavaScript, as already recommended; unchanged.
3. Drop litehtml: no grid, and no Arabic.
4. Before building it for real: a short second spike that puts a Blitz page into the GPUI panel (scroll, click a link, resize), with a window run only if the user says so for that run.

## Falco (tested 2026-10-12)

Question: could Falco, a from-scratch browser engine in Rust (github.com/poxk/Falco, v1.0.0, commit f0b4028 of 2026-08-11, MIT, about 77,000 lines), replace Blitz as the side-panel page viewer? Same method as above: pages drawn to pixel buffers and PNGs, no window, every run in a sandbox with the network off (`unshare -rn`) except the one test that needs a local listener. Falco was cloned outside this repository, built offline, and driven by a 100-line Rust program (`falco/src/main.rs`) that calls its library the way the app would. Pictures are `out/falco-*.png`. The commands and pages are in `falco/`.

### Short answer

Not usable as the viewer today. It is small and fast, but a page with no script at all draws blank, grid, flex and tables lay out wrongly, Arabic comes out in the wrong direction, its JavaScript fails on ordinary code, the engine fetches things on its own with no way for the app to refuse, and it panics, overflows its stack or hangs on pages that are only mildly unusual. Blitz stays the recommendation.

### Build and size

* The build read no scripts of its own: Falco has no `build.rs`. Its default features include `real-video`, whose crate (`openh264`) downloads a prebuilt Cisco binary at build time; that was turned off (`--no-default-features`).
* Offline build with `nice 19`: 1 min 34 s. 157 crates, pure Rust, no C or C++ compiler step, no system libraries beyond what the standard library needs. The command-line binary is 7.2 MB stripped with Falco's own release profile (which sets `panic = "abort"`). My test program was 10 MB unstripped.
* Licences of every crate in that build: MIT, MIT or Apache-2.0, Apache-2.0, BSD-2 or BSD-3, Zlib, ISC, CC0, Unicode-3.0, 0BSD. Nothing copyleft. Falco itself is MIT.

### Using it from Rust

It is a library crate, so the app would add a path or git dependency and call `render_with_base_url(html, css, options, base_url)`, which returns an RGBA canvas (as tall as the page) and the list of paint commands. No FFI, no helper library. Notes for a real use:

* The library does not pull `<style>` blocks out of the page; its command-line tool does, and the caller must do the same (a 15-line helper, copied in the test program).
* One render is one shot: it parses, runs the page's scripts once, lays out and paints. There is no live document, so no scrolling state, no hit-testing for links, no repaint after a script timer. Repainting the stored paint commands for a scrolled view is possible and cheap (below).
* It keeps global state (the base address for images), and its document types are `Rc`, so each page needs its own thread or process.
* It chooses fonts itself: one sans and one mono, from a fixed list of paths (DejaVu, Liberation under `truetype/`, and the Windows and Mac paths). None of them exist on this Arch install, so it fell back to the first `.ttf` it found under `/usr/share/fonts`, which here is a monospace programmer font, and every page came out in monospace. The pictures marked "liberation" use a test-only patch (an environment variable naming the sans font file) in my copy of Falco. There is no CSS font-family lookup, no per-character fallback, and no system font discovery.

### A page with no script draws blank

Falco's own command line did this too: `<div style="background:#0a0;width:100px;height:30px"></div>` gives a white image. The same page with `<script>var a=1;</script>` added draws correctly (`out/falco-blank-without-script.png` shows the blank one). The path taken when a page has no script loses the document. Every number and picture below was made with a harmless script appended (`HR_FORCE_SCRIPT=1`); without it, five of my eight pages came out empty.

### Quality, page by page (with that workaround)

| Page | Result |
|---|---|
| `article.html` | Text, heading sizes, quote bar and gradient box draw, but there is no body margin, long paragraphs do not wrap (they run off the right edge), the table is drawn as one cell per row, and the gradient box has a solid black bar under it where its shadow should be. |
| `layout.html` (grid and flex) | Grid is not applied: all five cards are stacked in one column, each with a doubled inner box. The flex row shows three boxes with only one label, overlapping. The tag pills have no text. Links are broken across lines. |
| shadows and gradients | Linear gradients, rounded corners and opacity draw. Box shadows draw as hard blocks or halos in the wrong place, the inset shadow and the repeating gradient do not draw, rotate and clip-path are ignored. |
| `arabic.html` | With the default monospace font no Arabic letter is drawn at all (blank gaps). With an Arabic font (Noto Naskh, via the test patch) the letters are shaped and joined correctly (it uses HarfBuzz through `rustybuzz`), but the paragraphs stay left-aligned and the order is left to right: no right-to-left layout, no mirrored table, list bullets missing. English words in an Arabic font show as empty boxes because there is no fallback. |
| `index.html` (the user's cut-off file) | Blank, as it should be. |

### JavaScript

Run through `falco --js-run`, one feature per file (`falco/js/`): arrow functions, template strings, classes, `const` and `let`, destructuring, spread, optional chaining, `Symbol`, JSON and regular expressions work. These failed: `Map` and `Set` are not defined, `Promise.prototype.then` is missing, generators fail, `[3,1,2].sort().map(...)` fails, `Date.prototype.toISOString` fails, an `async function` does not return a Promise, `2n ** 64n` gives `undefined`, `localStorage`, `fetch`, `XMLHttpRequest` and `WebSocket` are not defined in that mode, there is no `eval`. A 3-million-step loop takes 1.4 s. In page mode my DOM test (change text, change a style, add elements, a timer) died on the first DOM call with "undefined is not a function" and the page stayed as written (`out/falco-js.png`). The README lists 34 web APIs, and says itself that several are stubs.

### Memory and speed (900 x 1000 view, my test program, release build)

| | Falco |
|---|---|
| Process memory before any page | 3.8 MB |
| After parsing and drawing one page | 9 to 14 MB |
| After 20 repaints | 15 to 24 MB |
| Render, whole page (parse, style, layout, paint) | 3.5 ms (simple), 10 ms (article), 13 to 15 ms (grid, shadows) |
| Repaint of the stored commands into a 900 x 1000 buffer | 0.1 to 11 ms |

These are small numbers, but they are for pages that were mostly drawn wrongly.

### Network and file access: the engine fetches by itself

A page that asks for many things was served from a local listener on 127.0.0.1 inside a sandbox with only loopback (`falco/run_hostile.sh`). The listener saw, with no permission asked: `GET /script.js` for a `<script src>`, `GET /xhr` for an XMLHttpRequest, and `GET /img.png` for an `<img>`. The image request carried a made-up Chrome-on-Windows User-Agent; the script and XHR requests said `ureq/2.12.1`. An external script at a public name triggered a DNS lookup (it failed because the sandbox had no network). A `<link>` stylesheet, `@import`, `url()` backgrounds, `@font-face` and iframes were not requested in this run; `fetch` and `WebSocket` could not be tried because the script stops at the first missing object. There is no callback, allow-list or switch in the library for any of this. The only control is a Content-Security-Policy `<meta>` tag read from the page itself, which a hostile page will not include. So the safe way to use Falco would be to run it in a process that has no network (the `--sandbox` option exists but is a seccomp filter written by the project, not tested here), and that also blocks the images the app might want. The test of absolute `file://` addresses was inconclusive: Falco joined the page address and the file address into a broken URL, so nothing local was read.

### Crashes, hangs and size limits (`falco/crash_driver.py`)

Each page was run in a child process with a 20 s limit and a 6 GB memory cap, both through my test program (panics unwind) and through Falco's own binary (`panic = "abort"`). The parent survived every case, which is what a helper process is for.

| Page | Result |
|---|---|
| Cut off in the middle of a tag, cut off in the middle of CSS, the user's cut-off `index.html`, bad UTF-8 (test program), 3 MB attribute, upper-case non-ASCII in a `<STYLE>` tag | Drew something and exited normally. (Falco's own command line refuses a file with bad UTF-8.) |
| 4.7 MB of 70,000 `div` rows, 5.8 MB of 90,000 paragraphs | Panic at `paint/mod.rs:128` (index out of range on a very tall canvas); the abort build is killed by SIGABRT. |
| 20,000 table rows (340 KB) | Aborted after about 9 s. |
| 5,000 unclosed `div` elements | Stack overflow, process aborted (test program); the command line survived this one. |
| 100,000 nested `div` elements | Hung past 20 s. |
| `while(true){}` in a script, a script that fills memory | Hung past 20 s: there is no script time or memory limit. |
| A script that recurses forever | Stack overflow, aborted at once. |

A 5 MB page is a case the user asked about: Falco panics on it. Because Falco's own release profile aborts on a panic, an app that links it must either build it with unwinding and catch panics, or, better, run it in a helper process with a timeout and a memory cap, as above.

### Reading the code first

Before building I read the build files and the network code: no `build.rs`, no download step in the default build except the optional `openh264` (off), and the network calls are plain `ureq` calls spread over `lib.rs`, `net/`, `js_tjs.rs` and `web_runtime/` with no shared gate. The README's own status section says the HTML5 spec parser and the security module (SOP, CSP, seccomp, TLS chain) are "structurally complete, not wired into the renderer".

## Blitz and Falco compared

| | Blitz 0.3.0-beta.2 | Falco 1.0.0 |
|---|---|---|
| Memory with one page open | 20 to 26 MB whole process (includes the font index) | 9 to 14 MB after render, 15 to 24 MB after repaints |
| Added binary size | 14 to 18 MB stripped | about 7 MB stripped (command line) |
| First build | 139 s, 267 crates | 94 s, 157 crates |
| JavaScript | None, on purpose; scripts are never run or fetched | A home-made engine that fails on `Map`, `then`, generators, most DOM calls; no time limit |
| CSS | Grid, flex, shadows, gradients and tables drew correctly | Grid, flex and tables wrong; shadows partly wrong; no text wrapping; no margins |
| Arabic and right-to-left | Correct shaping, direction, mirrored list and table (one bullet fault) | Shaping works with an Arabic font; direction wrong; no fallback font |
| Network control | Every request goes to a function the app writes; all refused in the test | The engine fetches scripts, XHR and images by itself; no hook; no allow-list |
| Safety on bad input | Not stress-tested beyond the cut-off file; advised to run on its own thread | Panics on 5 MB pages, stack overflow, hangs on loops and deep nesting; needs a helper process |
| Blank-page bugs | None found | A page with no script draws blank |
| Fonts | System fonts through fontconfig | One sans and one mono from a fixed path list; none exist here |
| Maturity | Beta, actively developed upstream (DioxusLabs) | v1.0.0 from one author, own README lists stubs |
| Licence | MIT or Apache-2.0 (Stylo and a few crates MPL-2.0) | MIT, all dependencies permissive |

### Recommendation (not a decision)

Keep Blitz as the default page viewer (no script, the app decides every request) and Servo as the optional add-on for pages that need JavaScript. Do not use Falco: it fails the basics (blank pages, wrong layout, no right-to-left, no network control) and its safety limits are weak. If the user still wants to track it, the things that would change this are: pages without scripts drawing, a request callback for the app, a script time limit, and right-to-left layout. Re-check after its next releases; the cost of re-testing is small because the test program and pages are kept in `falco/`.

