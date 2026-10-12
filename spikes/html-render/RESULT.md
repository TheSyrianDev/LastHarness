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
