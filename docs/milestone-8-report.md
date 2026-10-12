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

## Leftovers done after the first commit

- **`show` tool.** The assistant can open text, a file or an image beside the chat. Core tool `show` (only in assistant and project sessions, read-only, no question): a title and either content or a path inside the session's folder, optionally a kind (markdown, code, text, image). The core reads the file under the folder rules (private places are refused, at most 400 KB, images pass as an absolute path) and sends a `show` event; the desktop app opens it in the side panel, the terminal prints it. Prompt cost: one line, about 17 tokens; the worst case with every tool line, now counted including the core's own, is 283 of 300.
- **Cost in the cloud resume warning.** With a price listed in the spending settings for the model, the warning says "about $X at your listed price of $P per million input tokens"; without, it says that no price is listed. Local endpoints never mention money.
- **KV cache saved per local chat.** See below.

### Saving the model's work on a chat (KV cache)

After each turn of a local, stored chat the core asks the local server to save its slot to a file named for the chat and the model (`lh-<chat>-<model hash>.bin`); a chat opened again from storage gets the file back before its first turn. Measurements are in `spikes/kv-cache/RESULT.md` (a 32k chat: six minutes to read again, 0.2 seconds to restore, 404 MiB on disk). Rules, all tested against a fake llama-swap: never for private chats (they have no stored conversation), never for a cloud endpoint, one file per chat (replaced on each turn), mode 0600, the whole folder kept under a cap (default 5 GB, oldest first), files deleted with the chat (closing a session alone does not delete them), restored only for the same model (the name carries the model), and the chat's resume warning says "The model's work on this chat was saved, so it should carry on quickly" when that is the case. If the server cannot save slots (cloud, LM Studio, Ollama, no save folder) or saves somewhere the core was not told about, the feature switches itself off for that run and says why in the activity log; a chat is never hurt by it. The activity log records sizes and times only.

It is controlled by three settings: `kvcache.enabled` (default on), `kvcache.dir` (the folder the server saves into; empty keeps the feature off, because a file the core cannot find is a file it cannot protect or remove) and `kvcache.max_gb` (default 5). With llama-swap the folder is the one above the per-model folders; the core finds its files below it.

## Frutiger Aero, redone to look like the real thing

The user said the first Frutiger Aero did not look like it, and lifted the plain-placeholder rule for this one look (they chose it, so it is made faithful; no logo or brand). The reference was the Omarchy theme `frutiger-aero-asadal-stock-1-day` on this machine (its palette and the pattern of its wallpapers). None of its photos are in the repository.

What it is now, all set in the theme file, not in code:

- **A painted sky.** `kind = "scene"` and a `[scene]` table. The app paints it in memory (about 90 ms once, kept while the theme stays): a multi-stop sky from deep blue to cyan, a hazy white horizon, a sun with a star and faint lens-flare rings, cumulus clouds with flat bases, rows of trees, a bright meadow and small daisies. Dark is a twilight sky with stars, a moon and dim clouds. Keys: `sky`, `ground`, `hills` (colour lists, top to bottom), `horizon`, `haze`, `haze_strength`, `sun_x`, `sun_y`, `sun_size`, `sun_color`, `flare`, `clouds`, `cloud_light`, `cloud_shade`, `stars`, `flowers`, `seed`.
- **Your own photo instead.** In `[background]` set `kind = "image"`, `image = "/home/you/Pictures/sky.jpg"` (a full path, or relative to the theme file) and `image_opacity`. The built-in files say so in a comment.
- **Glass panels.** `[glass]`: a translucent fill that is lighter at the top (about 66 percent to 46 percent in light), a white sheen over the top, a 1 px light line inside the edge and a pale blue line outside it, with a blue-tinted soft shadow. The title bar and the conversation sit on glass too, so text never lands on the deep blue.
- **Gel buttons.** `[gel]`: a gradient with a glossy top half, a rim light, a cyan glow while the pointer is over it (fades in over the animation time) and a darker veil while pressed. The main button runs blue to cyan; ghost buttons are bare text until hovered.
- **Gradient border on the panel in use.** `[focus]`: from `#1174a1` to violet `#6b60c4`, corner to corner, on the conversation, the chat list or the side panel, whichever was clicked last.
- **Bubbles.** `[motes]`: `count` soft bubbles drift up behind the panels. They move at 10 frames a second and only while the window is the active one. They are off when the theme's animation time is 0, when `count = 0`, and when the desktop asks for reduced motion (`LASTHARNESS_REDUCE_MOTION=1`, `GTK_ENABLE_ANIMATIONS=0`, or the desktop's own animation setting); reduced motion also turns hover fades off.
- Text is dark navy `#153141`, corners are 16 px, other looks are unchanged (no `[glass]`, `[gel]`, `[focus]` or `[motes]` means flat as before). The system prompt is unaffected.

Checked in the real window, light and dark (`target/gui-shots/fa-light-*.png`, `fa-dark-*.png`: start, chat with a real model, settings, memory, hover). Cost: with the bubbles on, about 4 percent of one core while the window is in use (a flat look idles near 0), memory about 100 MB. Turn them off with `count = 0` in the `[motes]` table. `crates/lh-gui/examples/scene_preview.rs` paints a theme's scene to an image file for tuning.

## Fixes after the user tried the Frutiger Aero build

- **Theme "stuck".** Cause found: a glass panel was given `position: relative`, which pulled the title bar's pop-up menus (theme and model) out of their floating place into the page flow, where the window cut them off. Under any look with glass the title bar theme button looked dead. Settings was never affected. Also, the menu stayed open after a choice, so the next click closed it instead of opening it. Fixed both: a panel keeps the position it is given, and choosing a look closes the menu. Checked once in the real window with real clicks through all eight looks from the title bar menu (each one saved to the preferences file); the Settings list was clicked through for the first look only. Tests: every look is saved, restored by `ThemeSet::index_of` and survives a restart (including a look whose file was deleted), and a panel keeps `absolute`.
- **Opened content not visible.** The app already revealed the right panel on a `show` event; it is now also guaranteed by a layout test (everything hidden, then show: the panel appears, the chat list stays hidden). The chat line is now a button ("Show again") that brings the panel back on that content, and an "Opened" tab sits in the title bar whenever something has been opened. Always-visible ways back: the "Chats" button for the list, and the Departments, Memory, Settings and Opened tabs for the panel.
- **Web pages.** A `.html` file (or `kind: html`) is no longer shown as raw text. The panel shows a card: "This is a web page", "Open in your browser" (xdg-open on the one file the core approved: an absolute path to an existing `.html` or `.htm` file; never a web address) and "View source". Servo is not built in; the decision stands: native viewers first, a real web page in the user's own browser, Servo later as an optional add-on. The core sends the resolved path with the source; the system prompt did not change (one more word in the tool's kind list only).
- **Note.** In that test the model wrote `/home/omar/index.html` in the user's home folder. It was allowed (assistant mode works from the home folder), but it is a file the user did not ask for by name.

## The chat tree (like Pi's /tree)

A stored chat is now a tree: every message says which message it answers, and the chat is "at" one of them. What the model reads is only the path from the first message to there (the active branch). Nothing is ever lost: going back and writing again starts a new branch beside the old one.

- **Store.** Migration 10: `messages.parent`, `chat_branch(conv, leaf)` (where the chat is; no row means the newest message) and `chat_labels`. Chats stored before this become one straight branch (the migration sets each message's parent to the one before it, lifting the "never edited" guard for that one step and putting the same guard back; a test checks it). The raw record stays append-only.
- **Core (`lhd`).** `GET /v1/chats/{id}/tree`; `POST /v1/chats/{id}/tree/move {node, summarize?}`; `.../tree/label`; `POST /v1/chats/{id}/fork {node}`; `POST /v1/chats/{id}/clone`. Choosing a message you said puts its words back in the input (`input` in the reply) and the chat goes to what came before it; choosing an answer continues after it. A loaded session follows the move: its history becomes the active branch, remembered notes are forgotten, and the saved KV cache is deleted (it describes a prefix that is no longer the chat). Tool calls are stored as `tool` lines so the tree can show them; they are never sent to a model again.
- **Summary of the branch you leave.** Setting `tree.summarize` = `ask` (default), `always` or `never`. With `ask` the core answers `needs_choice` and changes nothing until the client asks again with `summarize` true or false. A summary is written by the chat's model (a cloud model only after the privacy guard passes the text), stored as a `summary` line under the message you land on, and sent to the model afterwards as context. Nothing is asked when nothing is left behind.
- **Active branch only.** What the model reads, the resume warning's size and cost, the cloud privacy guard, the chat list's message count, `GET /v1/chats/{id}`, the end-of-chat review and the saved KV cache all follow the active branch. Keyword search over past conversations still finds every branch (it is the raw record).
- **Private chats** store nothing, so they have no tree (the tree calls say so).
- **Terminal.** `/tree` opens the tree: arrows move, Left/Right/Space fold, `t` hides tool results, `l` names a line, `f` forks, Enter goes there (and asks about a summary when something is left behind); `/clone` copies the branch. The chosen words come back in the input line. A fork or a copy is a new stored chat: the terminal has no chat list yet, so it tells you to open it from the desktop app.
- **Desktop app.** A "Branches" tab (Ctrl+Shift+T) shows the tree in the side panel: a line per message, the active branch and the chat's place marked, fold buttons, "Hide tool results", "Copy this branch to a new chat", and for the chosen line "Go here", "Name…" and "New chat from here". The summary question is a card with three buttons. The chat view shows the new branch and the message box gets the words back.
- Both front ends use the same list logic (`lh_client::tree::ChatTree`) and the same calls.
- **System prompt:** unchanged, no new tool and no new always-on text; the tree is not something the model sees, only a different history.

Tests: store (migration), memory (8: branches, plans, labels, fork, clone, sizes, delete), core API (9, plus one in the KV tests), client (5), terminal screen and keys (7), desktop state (5) and backend (1), and a terminal-client end-to-end test.

## The window redesign (in slices)

The user approved a calmer window (mock: "LastHarness Redesign"). It is built in four slices; each one leaves the app working.

**Slice a: icon rail and slim title bar.**
- A 52 px rail of icons sits on the side (chats, departments, memory, settings at the bottom). Chats shows or hides the chat list; the others open the side panel. The icons are lines drawn by the app, so no picture files ship. In right-to-left the rail moves to the right and everything mirrors.
- The title bar now holds only the chat's name, the branch it is on ("branch 2 of 3", only when the chat has more than one), the context size ("4,120 tokens") and the model. Over a department page it shows the department's name alone.
- Theme and text direction left the title bar. They are in Settings (the rail's last icon) and stay on Ctrl+T and Ctrl+Shift+R; the command palette (slice c) will list them too. The theme menu is gone with them.
- The side panel has its tabs along the top: Opened, Departments, Memory, Tree. Settings has no tab; the rail opens it and a tab appears while it is shown. The size controls sit on a line under the tabs.
- Accessibility: the rail is a navigation region; each icon is a button with a name, its key (`aria-keyshortcuts`) and a pressed state; the tabs form a tab list; the title bar is a banner. The names and keys come from `chrome.rs`, which is tested.
- Tests: `chrome.rs` (rail names in both directions, keys, icon shapes stay on the grid, pressed states, tabs, wording of the title bar), `ChatTree::branches` in `lh-client`.

## What is not done

See `.grill/parked.md`, section Milestone 8 (build session). Main points: agent-opened content is only opened by a button (the core has no "show this" tool yet, so nothing opens by itself); no web pages in the app (Servo waits); saving the model's KV cache per chat is parked, with measurements in `spikes/kv-cache/RESULT.md`; Orca was not run; drawing the app's own window frame for matching roundness on desktops that do not report a radius; showing the cost of a cloud resume (the core has no price list yet, so the warning gives tokens only); Windows, macOS and mobile are later milestones.
