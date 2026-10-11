# Spike: GUI toolkits

Question: which toolkit should the native LastHarness app use? The same small screen was built in Slint, egui, iced and GPUI, and later in Qt Quick (plain QML). Throwaway code, not product code. The look is each toolkit's plain default, nothing designed.

The screen: a sidebar list of models (from lhd `/v1/models`, tier words removed), a chat view that streams text from lhd over HTTP and SSE, a text input, and a Send button. The GPUI version also has Stop, a model switch inside one chat, and several chats (new, delete, switch, resume while lhd runs). `common/` is the shared client. `mock_lhd.py` is a stand-in server that streams a fixed Arabic and English text, so every toolkit sees the same load. `drive.py` opens the window, types, takes screenshots and reads PSS and CPU from `/proc`. `atspi_dump.py` prints the accessibility tree.

## Decision

The user decided on GPUI on 2026-10-11. This file records the facts behind it. The recommendation below was written before that decision and is kept for the record.

## Results (measured on this machine, native Wayland on Hyprland)

Window about 1880 x 1026. Memory is PSS of the app process. Streaming used the mock server at a steady pace.

| | egui 0.36 | Slint 1.18 | iced 0.14 | GPUI 0.2.2 | Qt Quick 6.11 |
|---|---|---|---|---|---|
| PSS idle | 118 MB | 73 MB | not measured | 60 MB | 130 MB |
| PSS streaming | 122 MB | 74 MB | not measured | 62 to 64 MB | 132 MB |
| CPU idle | 0.1% to 1.9% | 0.1% | not measured | 0.6% | 0.5% to 1.3% |
| CPU streaming | 2.4% | 2.2% to 3.8% | not measured | 2.2% to 2.4% | 0.3% to 1.6% |
| Release binary | 26.8 MiB | 30.4 MiB | 22.9 MiB | 37.3 MiB | runner plus libraries, 29 MiB of core libraries |
| Clean build | about 3 min | about 3.7 min | about 2.4 min | about 3.6 min plus fixes | no build (QML runs as is) |
| Arabic letters join | yes (font loaded by hand) | yes | yes | yes | yes |
| Right-to-left layout | no, left aligned | no, left aligned | partly | no, left aligned | yes, with a few lines of QML |
| Screen reader (AT-SPI) | not on the bus | list, items, button, field | no support | see below | list, items, field, button |
| Mobile (Android, iOS) | partly | yes | partly | no | yes |

Notes on idle CPU: a focused text box blinks its cursor, which keeps egui and Qt repainting. egui was 0.13% before the box had focus.

The iced run was cut off before it finished, so it has no numbers. The iced app does build and run.

## Arabic and right-to-left

- All five shape Arabic correctly and keep the right word order inside a line.
- None gives right-to-left layout by default. Lines stay left aligned and the sidebar stays on the left.
- Qt Quick was the only one where proper right-to-left was easy: `LayoutMirroring` mirrors the whole layout (sidebar on the right, buttons swapped, field direction) and a one-line rule right-aligned Arabic paragraphs. The cost was giving up cross-paragraph text selection in my version.
- GPUI: Zed's own editor has the same problems. The Zed tracking issue (#31102) is open, with fixes in review (`gpui_wgpu` #60115, `gpui_windows` #57237). A GPUI app would right-align lines and mirror the layout itself, and would depend on upstream for cursor and selection in right-to-left text.

## Screen reader

- Slint and Qt show a full tree on the accessibility bus: frame, label, list with one item per model, text field, button. I could not run Orca (not installed), so this is the tree only, not what is spoken.
- egui did not appear on the bus in my test.
- GPUI: AccessKit support was merged into Zed's GPUI on 2026-05-28 (pull request #56065), described there as mostly working. The `gpui 0.2.2` on crates.io dates from October 2025 and does not include it. Not tested.

## GPUI facts that matter for the build

- `gpui 0.2.2` is the last version on crates.io. `gpui-component` has to be pinned to 0.5.1 to match it (newer versions use a fork, `gpui-pre`).
- The build broke on an old dependency, `xattr 0.2.3` (pulled in by gpui's HTTP client). It needs a libc constant that newer libc removed. I patched that one crate locally (`gpui/vendor-xattr`).
- GPUI has no text input of its own. The test used `gpui-component` for the text box and buttons.
- Moving to a newer GPUI means taking it from Zed's git repository. That is a fetch, but a larger dependency than crates.io.

## Not done

- Theming tests: backdrop blur and translucent windows, gradients and shadows, button animation, a theme loaded from a data file and swapped live, background image or video. One related fact: a transparent Qt window on this Hyprland session shows the compositor's blur behind it. Nothing else here was measured for these.
- Tray icon for each toolkit. Not tested.
- Orca with real speech. Orca is not installed.
- Blitz (HTML and CSS without a browser engine, MIT or Apache, version 0.3 beta): not tested.
- Qt Quick 3D: not installed, skipped by the user.
- iced numbers, and a fair re-run of clean build times (the times above include failed first attempts).
- Native macOS and Windows UIs: not tested, no devices here.

## Rejected by the user during the spike

- Tauri and any web-view shell. Qt WebEngine with three.js worked (three.js r149 on WebGL 2.0) but the process tree used about 418 MB of PSS against 130 MB without it, so it was rejected.
- Closed-source or paid engines (Sciter, Ultralight).

## Recommendation (before the decision, a recommendation and not a decision)

Qt Quick was the best on right-to-left, screen reader, theming range, tray and mobile, and the heaviest on memory (130 MB) with a C++ build and a Rust binding. Slint was the best balance among the Rust toolkits (73 MB, accessibility tree, mobile). GPUI was the lightest (60 MB) with the nicest components, but weakest on right-to-left and accessibility today, and has no mobile. The user chose GPUI. To lower the risk, test the newer GPUI from git for AccessKit and right-to-left before building on it, and plan to write right-to-left layout in the app.

## Second round (2026-10-11): theming, Servo, roundness

### GPUI theming (a newer GPUI: `gpui-pre` 0.3.8 from crates.io, snapshot of Zed from October 2026)

A newer GPUI is on crates.io as `gpui-pre`, with platform crates `gpui-pre-platform` and `gpui-component` 0.7.1. No git fetch was needed. The spike is `gpui-theme/`; `theme_drive.py` drives it and `out/theme/` has the screenshots. Everything below was run in a real window on Hyprland (Wayland).

| Item | Result |
|---|---|
| Translucent window, blur | Works. Glass shows the desktop blurred behind it. The blur comes from Hyprland, not from GPUI: a control run with GPUI's blur request off looked identical. GPUI's own blur request needs the KDE blur protocol, which Hyprland does not implement, so on other desktops the result varies. Opaque themes stay opaque. |
| Gradients | Works (`linear_gradient`, any angle, per window and per surface). |
| Shadows, rounded and layered surfaces | Works (`BoxShadow`, `rounded`, nested panels with different opacity). |
| Animated button | Works: fade between two fills on hover, and a glow on click (restarts when its animation id changes). The six hover frames I captured all differed. Clicking itself could not be tested from here (no way to click). |
| Theme from a TOML file at runtime, swapped live | Works. The app polls the folder every 0.4 s; editing a file changed corner radius, accent and panel colour in the open window within 1.5 s, and restoring the file restored the look. Eight files ship as test data. |
| Background image | Works (`img` with `ObjectFit::Cover` and opacity, behind the gradient). Background video: not tested; GPUI has no video element. |
| Tray icon | Works with the `ksni` crate (StatusNotifierItem over D-Bus, no GUI toolkit needed). The item appeared in the watcher's list. Whether it shows depends on the bar supporting StatusNotifier. |
| AccessKit screen reader | Works at the tree level. With `role`, `aria_label` and `aria_selected` on elements, the AT-SPI tree showed: list "Themes" with eight list items, a text entry "Message", and buttons "Send" and "Mirror layout". The `gpui-component` text input appears as an entry. Orca itself is not installed, so what is spoken was not tested. |
| Arabic in the text input | Typing `مرحبا بالعالم hello` worked and the base direction switched to right to left. Cursor and selection behaviour in right-to-left text was not tested. The input's text colour was faint in the Glass dark theme because the component has its own colours; a themed input needs its colours set. |

### Mirroring the layout for right-to-left (how the app does it)

GPUI does not mirror anything by itself. In the spike the app flips the layout with a flag: `flex_row_reverse()` on the main row (so the sidebar goes to the right) and on the input bar (so Send and the mirror button swap sides), `text_right()` per paragraph chosen by the first letter's script, bubbles pushed to the other side, and Arabic labels. Result: sidebar on the right, user bubbles on the left, Arabic paragraphs right aligned, buttons swapped. What remains for the real app: mirroring icons that have a direction (arrows, chevrons), scroll bar side, keyboard shortcuts for left and right, and cursor movement in the input (the component's own behaviour).

### Roundness that follows the window

On Hyprland the window radius can be read (`hyprctl getoption decoration:rounding -j`, 15 here). With `match_window = true` panels take that radius and controls inside take the radius minus their padding (7 here), so the curves stay parallel. The screenshot `out/theme/match-window-radius.png` shows it. Other desktops give no such number, so the real app should draw its own window shape (client-side decoration on a transparent window) and use the theme's own radius everywhere; `match_window` stays for desktops that report it.

### Servo (`servo/`, throwaway)

The Servo crate on crates.io is now 0.7.0 (published 2026-10-05); 0.1.4 is an older line. License MPL-2.0. Rust 1.88 or newer. Built from the registry with the default features.

| Question | Result |
|---|---|
| 1. Render into or beside a GPUI window? | Yes, as a picture. A helper process (`servo-helper`) runs Servo with a software rendering context, paints into memory and writes a PNG about twice a second; the GPUI app shows the newest one as an image (`gpui-theme --embed URL`). This is not a live widget: no mouse or keyboard goes in, no smooth scrolling, a delay of up to half a second. Real interaction would need an input channel to the helper and a shared-memory frame path instead of PNG files. Servo can also render straight into a window it owns (`servo-window`). Nothing in the crate draws into another toolkit's GPU texture directly in a supported way. |
| 2. Memory, CPU, size, build time | Headless helper, one page: PSS 133 MB for a plain article, 140 MB for the Arabic page, 150 to 156 MB for the JavaScript page (peaks before settling). CPU about 0.5% idle for a still page, about 3% while the JS page animates a canvas; loading a page took 0.2 to 0.4 CPU-seconds. Release binary 146 MiB (110 MiB stripped) for the helper and 151 MiB for the window program, against 37 MiB for the GPUI app. A clean release build took about 10.5 minutes on this machine (12 threads, low priority), about 3 GB of build output. |
| 3. Rendering quality | A normal article (headings, table, quote, gradient box with shadow and rounded corners): correct and good. A JavaScript page (promises, async, maps, BigInt, `Intl`, canvas animation, `fetch`, timers): all worked, the user agent reports Servo 0.7.0 as Firefox 153. Arabic and right-to-left (`dir="rtl"`, lists, table, input field): the layout is right (right-aligned, bullets on the right, table columns reversed, mixed lines ordered) but shaping has faults: some words lost their first letter or showed marks in the wrong place (for example the first word of a list item and some words in a paragraph). So right-to-left structure works and text shaping is not yet reliable. Pages were served from a local test server; no real website was loaded. |
| 4. Separate helper process, crash isolation | Yes. With the helper as a child of the GPUI app, `kill -9` on the helper left the app running; it noticed within half a second, showed the status, started a new helper, and the page came back (restart count shown). The helper is not killed when the app is killed or closed (an orphan stayed), so a real design must set the child to die with its parent. Servo's own `multiprocess` feature (sandboxed content processes) is on by default but ran everything in one process in these tests. |
| 5. Pop out into its own window | Yes. `servo-window` opened a normal window with the page (same engine, 120 MB PSS), scrolling works from the winit example. It is a separate program, not a window made by the GPUI app. |
| 6. Licence and native libraries | MPL-2.0 (file-level copyleft: changes to Servo's own files must be shared; linking from another licence is fine). Native libraries the helper links to: fontconfig, freetype, expat, libpng, bz2, brotli, zlib, libstdc++, libgcc. No GL driver library was needed for the software path. SpiderMonkey comes in as the `mozjs` crates. |

Recommendation (not a decision): ship Servo, if at all, as an optional add-on helper downloaded or built separately, never in the main binary, and keep the native viewers (markdown, code, tables, images, diagrams) as the main way to show agent output. Reasons: 110 to 150 MiB extra and about 130 to 160 MB of memory per open page; no input path yet; Arabic text shaping has visible faults; page-to-image is not a real web view. Wait for Arabic shaping to be fixed and for a GPU texture path before making it a real feature. Dropping it costs nothing today.
