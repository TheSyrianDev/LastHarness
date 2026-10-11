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
