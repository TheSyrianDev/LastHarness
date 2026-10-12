# Parked for Omar

Things that need the user. Agents add to this list instead of asking while the user is away. Nothing here is decided.

- **GUI and branding design.** The user wants to design these together with the planning session once back. Do not invent visual design or branding. Build only functional, unstyled front ends until then.
- **Cloud API run** for spike 1 (sends requests off the machine): needs a go-ahead.
- **Ollama or LM Studio run** for spike 1: neither is installed.
- **Arabic test set** for memory retrieval needs a native reader.
- **Old backup copy** `/run/media/omar/Seagate/LastHarness.git.corrupt-2026-10-10` can be deleted once the user agrees.

## Milestone 1 (added by the build session)

- **Commit and push permission.** The planning session relayed that checkpoints may be committed as the user and pushed to backup without asking. I could not confirm that from my own session, so I have not committed or pushed anything from the build session. The planning session did the commits. Please confirm the rule once, directly.
- **Tray icon.** The roadmap says the core "collapses to the tray when there is one". A tray icon is a visual thing, so I built no tray. The core keeps running without a front end; the icon waits for the GUI design.
- **Start at login.** Not installed. A systemd user unit is the plan (`lhd.service`); installing it changes the user's machine, so it waits for a yes.
- **Spend: unknown prices.** A paid model with no price entry asks in estimate mode and is refused in caps mode. Local and private-network endpoints are free. Day and month limits use UTC boundaries, not local time. All three are my defaults; change if wrong.
- **Spend: no usage report.** If a server sends no token counts, the ledger row has zero tokens and, for priced models, zero cost. It does not guess.
- **Shell commands.** A command the app cannot classify asks every time (not once per session). Bypass has one level only (no prompts); the three bypass levels in the spec need the "hard locks" to exist first, which come with memory privacy in Milestone 2.
- **Compaction.** Runs between turns only, not in the middle of a long tool loop. When the server reports no context size, it starts at 24,000 tokens.
- **Web search.** Only SearXNG is built (checked with a fake server, not a real instance). The paid search API, the subscription search and TinyFish need their API details and keys, so they wait.
- **Web fetch.** Local and private network addresses are blocked for agents on purpose. Say if you want an allow list.
- **Capability probing.** Works for llama-swap and llama.cpp (tested live). Ollama and LM Studio probing is not written; neither is installed here to check.
- **Authentication listener.** The bearer-token listener is what exists. Pairing by PIN and device keys belong to Milestones 6 and 9, so "paired devices wake an idle core" is not built yet.
- **Image and video generation.** See below once done.
- **Video generation.** Not built. There is no common video endpoint across providers, and I could not check OpenRouter's video API from here. Images work through the OpenAI-style `/images/generations` endpoint (tested with a fake server only, not against a real image model).
- **Image prices.** Add `image = 0.04` (dollars per image) to a `[[price]]` entry in the spend settings. A paid image model with no price asks or is refused like any unpriced model.
- **Per-model compatibility overrides.** The compatibility table is detected per provider. A per-model override the user can edit is not built yet.
- **Exit test, cloud half.** Milestone 1 says "a real task finished on a local model and on a cloud model, same session format". The local half passed (see docs/milestone-1-report.md). The cloud half sends requests off the machine and waits for a go-ahead and a key.

## Milestone 2 (added by the build session)

- **Over-cautious privacy judging.** On the real run (Tiel-Coder-35B as the local judge), "Lives in Berlin" and "Prefers short plain answers" were both judged sensitive, because the rule is that an unsure model counts as sensitive. That is safe but noisy: those notes would never go to a cloud model. The review screen lets you flip each one with `p` before accepting, and `lh memory normal <id>` flips it later. Decide whether a place of residence or a style preference should count as sensitive by default.
- **Two privacy classes only.** Notes are Normal or Sensitive. Sensitive goes only to a model on this machine. The trust level "own cloud account" currently behaves like "any cloud". The spec has three trust levels but no third class of note, so I did not invent one.
- **"Forget" deletes for good.** The `forget` tool and `lh memory delete` remove the page, its older versions and its text in the history (the fact that something was deleted stays). Archive is the gentler option and is what a changed fact uses. Say if "forget" should archive instead.
- **Incognito.** No memory is read or written, no conversation is stored, and review is refused. The activity log still records tool names and permission answers, without any text, as a security record. Say if incognito should leave no log at all.
- **Embedding model is not bundled.** Meaning-based search works when `lhd` is given a llama.cpp server and the EmbeddingGemma 2 file (`--embed-server`, `--embed-model`), or an embedding endpoint. Without them, search falls back to keywords only. Shipping the model and server with the app belongs to packaging in Milestone 11.
- **Hybrid weights.** Keyword weight 0.35, meaning weight 1.0. On the small test set (8 paraphrase questions per language, real EmbeddingGemma 2) keywords alone found 3/8 English and 5/8 Arabic in the top 5, and the hybrid found 8/8 in both. Tuned on a tiny set; revisit with real use. A native Arabic reader still needs to check the Arabic test texts.
- **Review editing in the terminal.** The review screen can accept, drop, postpone or flip sensitivity. Editing a suggestion's text happens after accepting, by editing the Markdown file or with the API.
- **Folderless sessions and projects.** Pages of the projects layer attach to the project folder the session was opened in. Folderless sessions have no project.
- **Maintenance on idle.** `--maintain-when-idle` runs the deterministic checks only (duplicates, broken links, stale pages). The contradiction check needs a local model and is on demand.

- **Question for the user: "clippy".** The user told the coding session they had given "clippy as a reference model to embedding models". Nobody knows what this refers to (clippy is only the Rust linter the agents run). Ask the user what they meant.

## Milestone 3 (added by the build session)

- **Near-duplicate suggestions.** Two suggestions that say the same thing in different words can both appear, because they are compared by shared words, not by meaning. In the real run three sessions left two suggestions waiting. Matching by meaning (the embedding model) would merge them; it is not wired in yet.
- **Style note or rule.** The model decides whether "no bullet points" is a style note or a rule. On both Tiel-Coder and Qwen it chose a style note. Both kinds work and can be rolled back; nothing needs deciding unless you want formatting wishes to always be rules.
- **Qwen versus Tiel.** On Qwen3.6-35B (the model AGENTS.md now names) the learning suggestions and the memory review parsed fine, with a 3000 to 4096 token limit. The one difference: for "Lives in Berlin" Qwen's privacy judge said normal where Tiel's said sensitive. Both parsed cleanly. No switching back was needed.
- **Rules and the style profile live in the database, not as Markdown files.** They can be listed, edited and rolled back with `lh learn ...` and the API, and every version is kept. A Markdown mirror that you could edit by hand (like memory pages) is not built.
- **Whole-state snapshots.** Rollback is per item (each rule, the style profile, each skill has its own versions). Restoring everything learned to one earlier moment is not built.
- **Skills cannot be switched off.** They can be rolled back or deleted. A skill only reaches a model if it is valid; broken ones are listed with the reason.
- **When learned things take effect.** Rules, the style profile and the skill list are put into a session's first message, so a change applies from the next session. This keeps the start of every prompt identical for the server's cache.
- **Incognito.** Rules, style and skills still apply (they are how you want things done), but nothing is noted, suggested or learned.
- **Roles.** A rule can name a role, but roles arrive in Milestone 4, so none is set yet.
- **Skill suggestions only from a local model.** The record of a long task includes tool arguments, so skills are suggested (and improved) only when the model runs on this machine, and only after a task of at least 6 tool calls (`--skill-min-calls`).
- **Other skill folders.** Skills in `~/.claude/skills` or inside a project folder are not loaded. The format is compatible (the parser reads the real Omarchy skills), but loading project skills needs the folder-trust step that comes with Milestone 4.
- **Habit threshold.** 3 sessions by default (`--learn-threshold`); the model confirms that a repeated wish is real before anything is suggested.
- **Embedding server and the GPU.** Found and fixed: the embedding server (a CUDA build) crashed when the chat model filled the 4 GB GPU. It now runs with the GPU hidden. Worth knowing if you ever see it fail on another machine.
- **AGENTS.md reading order** still says "Milestone 0 is next" in the list under "Read first". The "Next step" section is current.

- **Seagate drive was not cleanly unmounted** (kernel: "exFAT-fs (sda1): Volume was not properly unmounted. Some data may be corrupt. Please run fsck."). This damaged the backup repository twice. Both times the local repository was intact and the backup was rebuilt from it. Two damaged copies are left on the drive (`LastHarness.git.corrupt-*`). Suggested fix, run by the user: unmount the drive, run `sudo fsck.exfat /dev/sda1`, then delete the damaged copies. Always unmount before unplugging. exFAT has no journal, so a second copy elsewhere would also be safer.

## Milestone 4 (added by the build session)

- **Project leader is the department's lead, not a clone of the assistant.** `start_project` makes the folder and runs one department in it. That department's lead plans and calls its members (which can be sub-departments), so a project can use several departments by nesting them. A separate "CEO" that calls several departments in turn is not built. The lock holds because workers never get `start_project` or `call_department` and cannot reach outside the folder without asking.
- **Permission requests do not stop at each level.** A worker's request goes straight to the user (or to a "no" in away mode). The spec says each level accepts, rejects or passes up. The middle levels would need a rule for when a lead may say yes; I left them out so that no worker can approve itself or a peer. Decide whether leads should ever accept on their own.
- **Away mode is deliberately blunt.** While you are away, a plan needing approval is approved (with the decision logged), and every permission question is answered no and logged. The step that needed it is set aside and listed when you return. The spec lets the assistant decide using what it knows about you; that needs a model call and a way to check "sensitive category", so it waits for your word. Sudo is never used while away because the answer to the question is no.
- **Plan-first is the default.** The first plan of every goal waits for you. Sub-department plans and steps inside it do not ask again.
- **Check-ins stop at milestones the lead names.** A plan with no milestones never pauses under "check-ins".
- **Roles are capped at 600 characters of instructions.** Longer is refused, with the reason, so a role cannot grow the prompt of every step it runs. The lead's brief carries the detail.
- **Worker prompt.** Base prompt, the role's own line, and one user message (the lead's brief plus what earlier steps returned). No history, no rules text, no memory dump. Measured: the base prompt is 139 tokens typical and 189 with the default tools; a worker adds at most 600 characters (about 170 tokens) of role line. Workers recall memory with their tools if their role allows it.
- **Rules and the style profile do not reach workers.** They are part of the assistant's first message, and workers get only the base prompt and a role line, to keep prompts small. A rule can name a role (the field exists) but nothing reads it for workers yet. Say if the user's style profile should go to writer roles.
- **Model types start empty.** The three stock types have no model until you set one (`lh team type set mid-tier <model>`). A run that needs an empty type stops with a message saying so.
- **One endpoint.** The core talks to one endpoint, so "select and operate models within registered providers" and "agents propose new providers and keys" are not built. Models are chosen by type or by name from what that endpoint lists. Loading and unloading is left to llama-swap (it loads on demand and the idle manager releases it).
- **MCP.** Local servers (stdio) and remote servers (HTTP, JSON or event stream) work, tools only. Adding a server shows the exact command or link and nothing runs until you approve. Every MCP tool call asks. What goes to a remote server is checked for sensitive text first. A key for a remote server is read from an environment variable you name, so it is never stored in the config. Not tested against a real hosted server (Higgsfield needs an account), only against mock servers. OAuth is not implemented.
- **Folder trust covers MCP servers only so far.** A folder may ship `.lastharness/mcp.json`. It loads after you trust the folder; "own" switches it off and stops the question. Project skills are not loaded at all yet (the skills folder is global), so the trust answer does not apply to them. Skill preview is listed in the roadmap and is not built.
- **Plugins are folders only.** Roles, departments and theme files, with a preview and no code. Installing from a link needs the network and a decision about where downloads may come from, so it is not built. Theme files are copied as they are; the theme format and anything that uses it wait for you and the planning session.
- **Themes, the Omarchy theme hook and the worker flow diagram** are not built (visual work).
- **Edit-plan in the terminal.** The API accepts an edited plan; the terminal offers approve or reject only.
- **Intermediate sizes.** The learned block (rules, style, skills index) and "about you" are not capped: rules are 400 characters each with no limit on how many, the style profile 1500, about-you 1200. They could grow the first message beyond the base prompt budget. Not trimmed, as agreed; worth a limit if rules pile up.
- **Failures keep their trail.** A step that gives up reports every attempt (which worker, why it failed). That report goes to the caller and finally to you. An unreadable answer from the lead (for example no answer to "what now?") counts as retry, and an unreadable quality check counts as OK, so a confused lead never blocks finished work.

## Milestone 5 (added by the build session)

- **Question for the user: how should first run be presented?** `lh setup` is the step-by-step form (about 12 questions, Enter accepts each default). The other candidates are one-choice profiles (Careful, Balanced, Hands-off) with an advanced view, or just-in-time questions. Recommended answer: profiles for the first run plus just-in-time questions, as the spec leans. Nothing else depends on the choice; the settings list and `lh settings` stay the same.
- **Settings file versus flags.** A command-line flag beats the file, and the file beats the default. `--allow` rules and `--embed-*` options are still flags only (they are paths and programs, not choices). Say if those should join the settings list.
- **Changing the provider needs a restart.** The model address, name and key are read when `lhd` starts. The settings screen says so. Switching live would also mean re-deciding how far the new endpoint is trusted with memory.
- **Permission mode (tiered or ask-all) and spending are read at start.** Only the team settings, crash-report choice and update choice apply at once.
- **Crash reports are only prepared.** A crash writes a small record (version, system, place in the code) and the preview endpoint shows it. There is nothing to send it to yet, and sending needs a decision about where. Updates (`updates.enabled`) are a setting only; there is no updater.
- **The key at setup is typed on screen.** A terminal cannot hide typing without a terminal library for it. The prompt says so. It is stored in the system secret store, never in the file. If the secret store is not available (no keyring service), setup fails with that message and nothing is saved.
- **Export folder holds private notes.** It is plain Markdown and JSON, not encrypted, and says so after exporting. Encrypting the export belongs with the subscription's sync work.
- **Import never replaces without being told.** Rules and notes already there are skipped; your own roles, departments and style stay unless named. A note imported with a different id from an existing similar note is added, not merged.
- **Skills and MCP servers are not in the export.** The roadmap names memory, rules and roles. Skills are folders and would be a natural addition; MCP servers hold commands and should be re-approved on a new machine anyway.

## Milestone 6 (added by the build session)

- **The browser door is off until you give it an address.** `web.listen` (or `--web-listen`). On a loopback address only this machine reaches it. A network address lets other devices connect, but each still has to pair with a PIN typed on this machine first. The local API on the loopback port is unchanged.
- **The certificate fingerprint is compared by eye.** A web page cannot read the certificate it is served over. The terminal shows the fingerprint and the pairing page tells you to compare it with the browser's certificate details. This is weaker than Sunshine's check, which can bind the PIN to the fingerprint; a native app can do that and the browser cannot.
- **Six-digit PIN, short life.** The PIN is only used to prove the pairing request is the one you are looking at. Limits: 5 requests waiting, 5 minutes, 5 wrong PINs lock the door for a minute. Someone who can intercept the HTTPS connection can still try PINs offline; the fingerprint check is the defence against that.
- **A paired browser is trusted for chat, memory, learning review, departments and most settings.** It cannot reach MCP servers, folder trust, export and import, plugin install or pairing, and cannot change provider, permission, web, image or spending settings. Everything else stays what the terminal can do. Say if a phone should be allowed more or less (for example no memory editing).
- **Starting a project or running a department from a browser** is allowed and follows the same permission questions, shown as buttons in the page. Bypass cannot be switched on from a browser.
- **Unstyled.** No colours, fonts, spacing or layout; the browser's defaults. Themes and the Omarchy colours wait for the visual design.
- **Only English.** Text is written in the page script, not in a translation file. Right-to-left and Arabic text shaping are checked in the native GUI milestone, but the page should get a translation layer before then.
- **Chat history is not listed.** The browser starts a conversation and shows its replies. Past conversations are searched through Memory (they are stored as notes) but there is no list of old chats to reopen.
- **Real-browser check is manual.** `crates/lh-core/tests/ui_smoke.mjs` needs a Chromium-based browser (it used Brave here). Not in `cargo test`.
- **Sessions live in memory.** Restarting the core signs every browser out; the stored key signs it in again at once, so you will not notice.
- **Revoked rows stay.** Removing a device keeps its row (marked removed) so the activity log still makes sense; the list shows removed devices only in the API.

## Milestone 7 (added by the build session)

- **No channel was tried against a real service.** Telegram, Matrix, Signal and Slack all need an account, a token or a linked phone number, so each client is tested against a mock of the service's published API (Telegram Bot API long polling, Matrix client-server sync, signal-cli's daemon, Slack Socket Mode). Real accounts will find differences. The first real try of each should be a short manual check: add, enable, send "hello" from an allow-listed account, send one from another account (nothing should happen).
- **Secrets are `env:NAME` only.** A channel token must be the name of an environment variable; a pasted token is refused. The `secret:NAME` form (the system secret store) is described in the help text but not wired for channels yet, because the core's service has no unattended way to prompt for the store. Say if channel tokens should use the secret store, and how it should be unlocked at start.
- **Allow-list by the service's own id.** Telegram: numeric user id (not the username). Matrix: the full id (@you:server). Slack: the member id (Uxxxx). Signal: the phone number. Names would be easy to impersonate.
- **A stranger's message is dropped silently.** No reply, no wake-up, no stored text. A count of how many were ignored is shown in `lh channels` so you can notice an attempt.
- **Lowest trust level.** A channel conversation never uses sensitive notes or rules (as for any cloud model), and a reply that looks sensitive is held back with a notice. The check ignores words with a slash (paths and links). It is a pattern check, so it can miss things and can hold back harmless text. Memory writes from a channel conversation are allowed (as in any session); say if channel chats should be incognito.
- **Modes.** A channel starts in `chat` (no tools). `assistant` mode allows tools; every permission question is sent to the chat as yes or no. A reply of yes or no answers the oldest open question. This means anyone who controls an allow-listed account controls what the assistant may do, under the same permission rules as at the terminal. Bypass is not offered here. Say if `assistant` mode should be refused for Slack.
- **Departments from the phone.** `/run <department> <goal>` works, with plan approval and questions by yes or no. Check-ins arrive as yes/no too. It follows the autonomy setting.
- **Matrix: unencrypted rooms only.** An end-to-end encrypted room cannot be read (that needs a full Matrix crypto library). Old messages before the first start are ignored. The assistant joins invitations only from allow-listed people.
- **Slack: direct messages only,** not channels or mentions, to avoid answering a whole workspace. It uses Socket Mode, so no public address is needed and each user makes their own Slack app.
- **Signal needs signal-cli running in daemon mode,** already linked to a number. LastHarness does not handle Signal's keys or registration.
- **WhatsApp:** out for good, as decided; nothing was written for it.
- **Channel settings live in the store, not the settings file,** because they hold a list of people. The browser cannot reach them (terminal only). They are not part of export.
- **Sending is logged by size only** (`channel_in`, `channel_out`: kind, number of characters, whether held back). Message text is not written to the activity log.

## Milestone 8 (added by the build session, from the planning session)

- **Resume a chat with a different model, after a warning.** A stored chat can be reopened with a model other than the one it started with. Before it continues the user is told what it costs: for a local model, that the first reply will be slow because the whole chat is read again, with the number of tokens of context the chat holds; for a cloud model, that it spends credits because the whole chat is sent again.
- **lhd needs an endpoint to resume or switch a stored chat.** It also covers resuming after an lhd restart (today sessions live in memory, and lhd has no way to list stored conversations or rebuild one). The answer must include the context size in tokens.
- **The privacy guard must still apply when a chat moves to a cloud model.** A chat that holds sensitive notes must not send them to a cloud model: the same outbound checks as for a new cloud session, applied to the whole stored history, not only to the next message.
- Open for the design: what the warning looks like, whether credits are estimated in money or only in tokens, and whether a switch to a cloud model with sensitive history is refused or the sensitive parts are dropped first.
- **Save the KV cache (prefill) per local chat, so a chat resumes faster.** Idea from the user, not built. llama-server has `--slot-save-path` and slot save/restore. To check before any design: whether llama-swap lets the core reach slot save and restore, whether a model swap keeps or drops the slots, how big a saved cache is for a 4k, 16k and 32k chat on Qwen3.6-35B, and restore time against re-reading the prompt. Rules so far: only for the same model file and settings; never for incognito chats; stored with the chat store's protections (file mode 0600); deleted when the chat is deleted or forgotten; counted as private data. Cloud models: we save nothing.
- **KV-cache save: automatic or manual?** Unanswered. Recommended answer: save automatically when a local chat is left idle, with a 5 GB cap for all saved caches (oldest removed first), and a setting to turn it off.
- **Does the cloud-model warning show an estimated cost?** Unanswered. Recommended answer: show the cost when the price of the model is known, otherwise show only the number of tokens that will be sent again.
- **Control roundness follows the window roundness, on every desktop (user, 2026-10-11).** Spike result: on Hyprland the window radius can be read (`hyprctl getoption decoration:rounding`, 15 here), and panels use it while controls inside use that radius minus their padding (7 here), so the curves stay parallel. Other desktops have no such setting to read. Recommended answer for the app: the app draws its own window shape (client-side decorations on a transparent window), so the window radius is the theme's own number and always matches; where the compositor draws the frame (Hyprland), a theme can say `match_window = true` to take the compositor's value instead. Built-in looks would set the radius per theme (Calm small, Matte medium, Glass and Frutiger Aero large).

## Milestone 8, build session (desktop app)

Each item says what the app does now and the answer I recommend. A "yes" accepts it.

- **Keyboard sends.** Enter sends the message and Shift+Enter makes a new line. Recommended: keep.
- **Deleting a chat asks twice** (the button turns into "Delete?"). A stored chat is removed for good, including its search entries. Recommended: keep; a trash with undo can come later.
- **Chat kinds.** Before the first message a chat is plain (no tools), an assistant (home folder) or a project (one folder), and plain chats can be private (nothing remembered). After the first message the kind is fixed. Recommended: keep.
- **Opening a stored chat** shows its history at once and asks only when you continue. Recommended: keep.
- **"Open in panel" is a button.** The agent cannot open things by itself yet, because the core has no tool for it. Recommended: add a `show` tool later that returns a title, a kind and the content, and let the app open it in the panel.
- **The Memory panel** changes pages (pin, sensitive or normal, archive) and answers suggestions, but does not edit text. Recommended: editing text stays in the terminal until the design talks decide how it should look.
- **Settings** change in place only when they are on or off, a choice or a number in a range; the rest are shown with the command to change them (`lh settings`). Provider and spending settings stay terminal-only as before. Recommended: keep.
- **The tray** has Quit only. Recommended: add Show when the toolkit can raise the window from the tray's thread.
- **Roundness.** The app uses the theme's own radius; `match_window = true` takes the window's radius where the desktop says it (Hyprland). Recommended (see the earlier item): draw the app's own window frame on desktops that report nothing.
- **Colours of the text box** follow the theme, but the toolkit's own text input has a fixed focus ring colour. Recommended: leave for the design talks.
- **Cloud resume shows tokens only,** not a price, because the core has no price list. Recommended: show a price when the endpoint reports one.
- **Binary size** is 64 MiB (46 MiB stripped). Recommended: turn on `strip` and thin LTO for release builds when packaging starts.
- **Chats list newest first;** the order does not change when a chat gets a new message. Recommended: move a chat to the top when it is used.

## Milestone 8 leftovers (build session)

- **KV cache and slotkeeper.** The per-chat cache works on a plain llama-server (0.12 s instead of 2.6 s for a 7k chat, measured) and does nothing for entries that run through the router's slotkeeper (57.5 s against 57.9 s on Qwen3.6), because slotkeeper restores its own snapshot into the slot and prefills again on each request. Recommended: set `kvcache.dir` only for entries with plain llama-server; or let the planning session decide whether slotkeeper should keep one session file per conversation (that tool lives in `~/AI/rig`, outside this repository).
- **The app's own window frame** is left as it is. Reason: on Hyprland and other compositors that round windows themselves, a frame drawn by the app would be rounded twice and fight tiling; on macOS and Windows the system draws the frame. Only desktops that give square windows and no radius would gain, and none of those is on this machine to check. Recommended: keep `match_window` for desktops that report a radius (Hyprland) and the theme's radius elsewhere.

## Frutiger Aero redo (build session, 2026-10-12)

- **The plain-placeholder rule is lifted for this one look.** The user chose Frutiger Aero and said the first version did not look like it; the planning peer relayed that no branding or logo is allowed but the look should be faithful. Done in `crates/lh-gui/themes/frutiger-aero-*.toml` and described in `docs/milestone-8-report.md`.
- **Bubbles cost about 4 percent of one core while the window is in use.** Recommended answer, taken: they ship on (five bubbles, 10 frames a second, only while the window is active, off for reduced motion or animation 0). The user can set `count = 0` in `[motes]`. If battery matters more than the look, flip the default to 0.
- **Photos.** The reference wallpapers are stock photos of unknown licence, so none is in the repository; a theme may point to a path of the user's own (`kind = "image"`).

## Chat tree (build session, 2026-10-12)

- **Memory and learning read only the active branch.** Recommended answer, taken. New learning signals come from what is sent, which is always on the active branch; the end-of-chat review reads the live history, which follows the branch; `GET /v1/chats` sizes and resume use the branch. Keyword search over past conversations still finds every branch, because it is the raw record. Change this only if you want search to skip other branches.
- **Branch summary: ask each time.** Recommended answer, taken: setting `tree.summarize` = `ask` (also `always`, `never`).
- **Private chats have no tree.** Nothing is stored for them, and a tree needs the stored messages. Recommended: leave it so ("incognito leaves nothing").
- **Tool results in the tree are short notes, not the real results.** The store only ever kept what was said; the tree stores one line per tool call ("name arguments → start of the result") so it can show and filter them. They are never sent to a model again, so after a move the model has lost the full tool results of the new branch (it keeps what it said about them). Recommended: accept.
- **A move rebuilds the model's history from stored text,** like resuming a chat, so the first reply after a move is slow on a local model unless the cache matches (it is dropped on a move). The warning is not shown for a move on the same model (nothing is re-sent to the cloud that was not already part of this chat).
- **The terminal cannot open a fork or a copy.** It has no chat list. Recommended: add `lh --resume ID` and a chat list to the terminal later, together.
- **Moving while a turn runs is refused** (the tree panel shows the core's message).

## Window redesign (build session, 2026-10-12)

- **The context meter shows a number, not a bar.** The mock draws a bar, but the app does not know the model's context size (the core compacts at a threshold it keeps to itself). A bar with an invented size would mislead. Recommended answer, taken: show "4,120 tokens" only. Later the core can send the context size with the usage event and the bar can follow.
- **The branches tab is called "Tree"** as in the mock (Arabic: the tree). The terminal still says `/tree`.
- **Settings has no tab in the side panel.** The mock lists four tabs (Opened, Departments, Memory, Tree) and a Settings icon on the rail. Settings opens in the same panel from the rail, and a fifth tab appears while it is shown. Recommended: keep; a page of its own can follow if the settings list grows.
- **Opened shows as a tab even when nothing is opened** (as in the mock); it then explains how to open something.
- **"Always for this chat" lives in the window, not the core.** The core only knows yes and no (and its own "writes ask once per session" rule). Recommended answer, taken: the window remembers the tool and the reason for one chat until the app closes, and never offers it for deleting, installing or running as root. If it should be saved or enforced by the core, that needs a core change and a decision about where it is stored.
- **Thinking counts as a step** in "Worked for 14 s, 3 steps". Recommended: keep, since it is part of the work.
- **Questions sit under the conversation, not inside the scrolling feed,** so a permission question is never scrolled out of sight. The mock puts the card at the end of the feed. Recommended: keep it fixed under the feed.
- **The message box is not the mock's field yet** (mode chips, a round send button). That was not among the four approved items. Recommended: do it with the dock and motion slice if wanted.
- **The palette searches names, not message text.** Recommended answer, taken: names of chats, models, looks and places only. Searching what was said needs the core's search and a result list; add it later if wanted.
- **The palette has no "recent" section.** With nothing typed it lists places first, then chats in list order. Recommended: leave until the list is long enough to need it.
