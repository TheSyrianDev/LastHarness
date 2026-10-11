# Milestone 8 report: the desktop app

Written by the build session while the user was away. Plain placeholder visuals only: no logo, no name styling, no brand colours. The look is the plain default (Calm) plus three more built-in looks the user asked for.

## What exists

New crates `lh-client` (the API client, moved out of `lh-tui`, which re-exports it) and `lh-gui` (library `lh_gui` for everything that does not draw, binary `lastharness-gui` for the window). GPUI is the toolkit, as the user decided. The app is a thin client of the core, like the terminal and the browser page.

**Window.** Three parts, each hideable and resizable: the chat list on the left, the chat in the middle, a right panel (departments, memory, settings, opened content) that can be shown, hidden, made wider or narrower, and expanded to fill the window except the chat list. A department's own page replaces the middle while the chat keeps running behind it; it opens from the chat list or the right panel.

**Chats.** New chat, switch, delete (the delete button asks twice), newest first. A chat can be plain, an assistant (home folder, asks before changes), a project (one folder), or private (nothing remembered or stored); the kind is chosen before the first message. Streaming answers in markdown, "Show thinking" for reasoning, tool calls as small cards with details, permission questions as Allow and Deny cards, a Stop button that turns the Send button red while the model answers (Enter sends, Shift+Enter makes a new line), and the number of tokens of context in the top bar.

**Models.** The model menu shows each model once by name, without the router's tier words (fast, balanced, max, low, native, stock, long, high); the app picks the entry itself ("balanced" if there is one). Choosing another model in a chat that has started asks first (see resume below), then the chat goes on with the new model.

**Stored chats and resume.** The chat list also shows the chats the core has stored, so after closing the app, or restarting the core, they are still there. Opening one shows its history at once; continuing asks first: "The first reply will be slow: the model has to read the whole chat again, about N tokens of context." A cloud endpoint gets "This sends the whole chat again to the cloud model ... and that spends credits." Ctrl+Enter continues, Escape cancels. Checked in the real window across a restart of the core: the chat came back, the warning showed, and the next question ("And its biggest moon?") was answered with the earlier context ("Ganymede").

**Departments.** The list, a goal box, runs with the plan, each step and its state, the questions waiting for the user (approve the plan, carry on at a milestone, allow a tool) as buttons, the outcome as markdown, and Stop. Runs still going when the window opens are picked up again (the core replays a run's log).

**Memory and settings.** Memory: the pages, suggestions waiting for an answer (Remember this, Not this), and per page Read, Pin, Make sensitive or normal, Archive, Open in panel. Settings: the core's own settings with on and off, a choice or a number in a range changed in place (others are shown, with the command to change them); the app's own appearance section on top (looks, direction, chat list width, problems found in theme files).

**Opened content.** "Open in panel" on a long reply, a tool result, a memory page or a department's result opens it in the right panel with a native viewer: markdown (including tables), code with line numbers, an image file, plain text. It can be made larger or popped out into a window of its own (same look). The keyboard does the same (Ctrl+Shift+L opens the last reply, Ctrl+Shift+P pops out). Web pages are not drawn: the user rejected web views, and the Servo test said to wait.

**Looks.** Eight TOML themes ship (Calm, Matte, Glass, Frutiger Aero, each light and dark); Calm light is the default. A theme sets colours, background (solid, gradient or image), roundness, blur, shadow and animation time. Your own files go in `~/.config/lastharness/themes/` and changes show within half a second, a file with the same name as a built-in look replaces it, and a file with a mistake is reported with the key that is wrong. Glass and Frutiger Aero use the translucent window and, on Hyprland, the desktop's blur. A theme can ask for `match_window = true` to take the window's own corner radius where the desktop reports it (Hyprland does); controls inside panels then use that radius minus their padding. Buttons fade between resting and hovered looks over the theme's animation time.

**Right to left.** Direction is Auto (from the desktop language), Left to right or Right to left. Right to left mirrors the whole layout (list on the right, panel on the left, buttons swapped), right-aligns paragraphs that start in Arabic or Hebrew, and switches the main labels to Arabic. Typing Arabic into the message box worked and the answer came back right aligned. Not done: mirroring of direction-bearing icons (there are none yet) and cursor behaviour in mixed text, which belongs to the toolkit's input component.

**Keyboard and screen reader.** Every main action has a key: Ctrl+N new chat, Ctrl+B chat list, Ctrl+Shift+D, M, S, O the panels, Ctrl+Shift+E expand, Ctrl+T next look, Ctrl+Shift+R direction, Ctrl+. stop, Ctrl+PageUp and PageDown between chats, Ctrl+Enter and Escape for the resume question. The AT-SPI tree shows named buttons, the chat list and its items, the conversation as a log, the message box as an entry and the Send button; icon-only buttons have spoken names ("Delete chat"). Orca itself is not installed here, so what is spoken was not tried.

**Tray.** A tray icon through the StatusNotifierItem protocol, with Quit (it registered on this desktop). Show is not offered: a tray cannot raise the window from another thread yet.

**Changes to the core (lhd).**
- New API for stored chats: `GET /v1/chats`, `GET /v1/chats/{id}`, `DELETE /v1/chats/{id}`, `POST /v1/chats/{id}/resume` (warning first, then confirm; a different model is allowed; the same model in a loaded session is just continued).
- The privacy rule holds: on a cloud endpoint a chat that repeats a sensitive note is refused (409) with an explanation, and nothing is opened.
- Migration 9: a stored chat remembers its model, mode and a short title.
- Bug fixed: session ids restarted at s1 after every core restart, so a new chat was stored inside an old conversation. The counter now starts after the largest stored id.

## System prompt size

The GUI adds nothing to the prompt. The one change in this stretch is "built by LastSoftware" in the first line, the user's request: 146 tokens typical (was 139), 195 with the default tools (was 189), 240 with every tool line (was 234); limit 300. The test that keeps this under the limit is green.

## Numbers

Measured on this machine (Wayland, Hyprland, release build).

| | |
|---|---|
| Memory, app idle (PSS) | 114 to 126 MB |
| Memory while streaming | about 120 MB |
| CPU idle | 0.0 to 0.4% |
| CPU while an answer streams | about 5% |
| Binary | 63.9 MiB (46.2 MiB stripped); `lhd` 23.4 MiB |
| Clean build of the app and its toolkit | about 6 minutes (4 minutes once the toolkit is built) |

The idle memory is about twice the toolkit spike (60 MB) because the real app carries the text input, the markdown viewer and the TOML and layout code. Still lighter than the Qt window (130 MB) and well under a web view. Stripping is not turned on in the workspace; doing so would take about 18 MiB off the app.

## How it was checked

Whole workspace: all 361 tests pass (`cargo test --workspace -- --skip real_embeddinggemma`), `cargo clippy` is clean on the changed crates.

- `lh-core/tests/chats_api.rs` (5): list, title, delete, restart without reusing an id, continue after a restart with the warning first, another model, the privacy guard on a cloud endpoint.
- `lh-gui` unit tests (46): themes (parsing, errors, built-ins, user folder and live change), chat model (streaming, tools, questions, every way a turn ends, chat list), department run model, layout state, right-to-left rules, models list, settings kinds, preferences, config, content viewer choice.
- `lh-gui/tests/backend.rs` (8), against a real core router with a scripted model: connect and stream, unreachable core, Stop in the middle and carry on, model switch, stored chat load, resume and delete after a restart, a department run followed and answered to the end, a private chat leaving nothing and a project chat remembering its kind, a permission question answered yes before the file is written.
- In a real window (`lh-gui/tests/gui_drive.py` and `gui_resume.py`, screenshots in `target/gui-shots`): the calm default, chat with a real model, every panel, Glass with the desktop blurred behind it, Frutiger Aero, mirrored right to left with an Arabic chat, live reload of a theme file, opened content and its pop-out window, the resume flow, the tray, the AT-SPI tree.

## Decisions made on the way (to confirm; they are also in `.grill/parked.md`)

Enter sends and Shift+Enter makes a new line; delete asks twice; a chat is plain, assistant, project or private, chosen before the first message; opening a stored chat shows it at once and asks only when continuing; "Open in panel" is a button, not automatic; the memory and settings panels change what is safe to change in place and show the command for the rest; the tray has Quit only; the window's roundness comes from the theme (the app does not draw its own window frame).

## What is not done

See `.grill/parked.md`, section Milestone 8 (build session). Main points: agent-opened content is only opened by a button (the core has no "show this" tool yet, so nothing opens by itself); no web pages in the app (Servo waits); saving the model's KV cache per chat is parked, with measurements in `spikes/kv-cache/RESULT.md`; Orca was not run; drawing the app's own window frame for matching roundness on desktops that do not report a radius; showing the cost of a cloud resume (the core has no price list yet, so the warning gives tokens only); Windows, macOS and mobile are later milestones.
