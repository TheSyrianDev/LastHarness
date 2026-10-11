# Milestone 7 report: messaging channels

Written by the build session while the user was away.

## What exists

New crate `lh-channels`, plus `channels.rs` in `lh-core` and `lh channels` in `lh-tui`.

- **A channel interface.** Two jobs: wait for messages, send replies. Everything else is decided once in the core, the same for every channel. Four clients: Telegram (Bot API, long polling), Matrix (client-server API, long-polling sync, unencrypted rooms), Slack (Socket Mode over a WebSocket, direct messages only) and Signal (through `signal-cli` in daemon mode). WhatsApp is not built, as decided.
- **Opt-in, off by default.** A channel is added off, whatever it is told. It can be switched on only after the user has been shown the warning for that kind of channel and confirms it, and only with an allow-list, a model, and settings that really work (it checks now, not later). Tokens are written as `env:NAME`; a pasted token is refused. Each kind's warning says the service can read the messages, that LastHarness cannot protect them, and what is never sent; Slack's adds the workspace owner, message history and the advice to create your own app.
- **Only people on the list.** The list uses the service's own ids. A message from anyone else is dropped: no reply, no wake-up of the idle core, no text kept (a counter shows how many were ignored).
- **Wakes the idle core.** An allowed sender's message reaches the core through its normal local API, so it wakes an idle core exactly as any client would; a stranger's message makes no call at all (tested with a core that had gone idle).
- **Lowest trust level.** A channel conversation never uses sensitive notes or sensitive rules or the sensitive part of the style profile, as for any cloud model. A reply that looks sensitive is held back and the chat is told so. Sending and receiving are in the activity log by size only.
- **Approvals from the phone.** In `assistant` mode a permission question arrives in the chat ("... wants to use write_file: ... Allow? Reply yes or no."), and yes or no answers it. `/run <department> <goal>` gives a goal to a department: the plan arrives for a yes or no, milestones too, then the result. Also `/new`, `/status`, `/stop`, `/help`.
- **Terminal.** `lh channels` (list with status), `kinds`, `add`, `edit`, `enable` (shows the warning and asks), `disable`, `remove`.

## System prompt size

Unchanged: 139 tokens typical, 189 with the default tools, 234 with every tool line (limit 300). A channel conversation uses the same base prompt.

## How it was checked

279 automated tests pass across the workspace and `cargo clippy` is clean.

- Each client against a mock of its service's API: what is returned and what is not (own messages, bots, receipts, history, encrypted rooms, non-direct Slack messages), the position moving forward so nothing repeats, acknowledgements, sending, bad credentials, a service that is down.
- The whole path with a mock Telegram, the real core and a scripted model: off until acknowledged; stranger ignored without waking the core; an allowed person answered and the next message continuing the same conversation; a sensitive reply held back; sensitive notes and rules missing from the channel's prompt while ordinary ones are present; a permission question sent to the phone and a "no" ending it with the file not written; a department goal planned, approved and answered from the chat; `/new` and `/help`.

## What is not done

See `.grill/parked.md`, section Milestone 7. No channel was tried against a real service; that needs accounts and tokens. Tokens must be environment variables for now (the system secret store is not wired for channels). Matrix end-to-end encrypted rooms cannot be read.
