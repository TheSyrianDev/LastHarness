# Milestone 6 report: browser UI

Written by the build session while the user was away. Functional and unstyled on purpose: no colours, fonts or layout were invented.

## What exists

In `lh-core`: `web.rs` (the browser's door), the page and script in `crates/lh-core/web/`, `lh devices` in `lh-tui`.

- **Served by the core over HTTPS.** Off by default; `web.listen` (or `--web-listen`) turns it on, for example `127.0.0.1:7843`. Use the machine's network address to reach it from a phone. The core makes its own certificate the first time (kept 0600 in its data folder, remade after 300 days). Pages carry a strict content policy (own origin only, no framing, no inline script), no caching and same-site referrer rules.
- **Pairing, as in spec 12.1.** The browser makes a P-256 key pair whose private half cannot be exported, shows a 6-digit PIN, and sends its public key plus an HMAC of the key made with the PIN. You type `lh devices accept <PIN>` on the machine; the machine finds the request the proof belongs to and pairs it. Safeguards: at most 5 requests waiting for 5 minutes; 5 wrong PINs drop everything and lock the door for a minute; two requests matching one PIN pair neither.
- **Signing in.** A fresh challenge is signed with the key (ECDSA, checked by the core), giving a 12-hour cookie (`Secure`, `HttpOnly`, `SameSite=Strict`). No password. The cookie dies at once when a device is removed. A nonce works once.
- **What a paired browser may reach.** The screens' own areas only. It cannot reach MCP servers, folder trust, export and import, plugin install, pairing itself, or the memory import and export paths. A change must carry an `Origin` of this site. From a browser, settings for the provider, permissions, web, media and spending are refused with a message to use the terminal.
- **The screens.** Chat (model, folder or plain chat; streamed replies; permission questions as buttons), Memory (search, view, edit, archive, delete), Review (notes to remember, what it has learned, rules in force with on/off), Departments (give a goal, approve the plan, check-ins, permission questions, who is working with pause/resume/stop, away mode), Settings (every setting with its help text and whether it applies now, and the list of paired devices with Remove). Pairing screen when the browser is not paired (pairing must first be opened on the machine with `lh devices pair`).
- **Accessible by default.** Landmarks, a skip link, a language, real labels on every field, real buttons, a status line and logs that announce changes, focus moves to each view's heading, questions are alert dialogs with focus on the first choice, everything works from the keyboard. All server text goes in as text, never as HTML.
- **The terminal side.** `lh devices` shows this machine's certificate fingerprint, paired devices and anyone waiting; `lh devices accept <PIN>`, `lh devices remove <id>`.

## System prompt size

Unchanged: 139 tokens typical, 189 with the default tools, 234 with every tool line (limit 300). The browser sends the same requests as the terminal and adds nothing to any prompt.

## How it was checked

- 6 tests over real HTTPS: the pages and headers, the API refusing everything without a signed-in device (and the machine's file token not working over the network), the full pairing and sign-in, replayed and foreign signatures, the allowed and refused paths, Origin checks, revoke, lockout, shared PIN, certificate reuse.
- **Interop with a browser's own cryptography.** A Node script runs the exact WebCrypto calls the page makes (key generation, non-exportable private key, HMAC proof, IEEE P1363 signature) against the real server. Run by `cargo test` when Node is installed.
- **The real page in a real browser.** `crates/lh-core/tests/ui_smoke.mjs` drives the page in headless Brave (a Chromium browser) over the DevTools protocol against a running core and a mock model: it pairs, takes the PIN from the screen, accepts it on the machine, visits every screen, chats and watches the streamed reply, gives a department a goal and approves the plan, changes a setting, is refused a dangerous one, checks the landmarks and focus, reloads and is signed in again by the stored key, and is sent back to pairing after the device is removed, with no page errors. It found a real bug (over HTTP/2 the request has no `Host` header, so the same-site check failed). It is not part of `cargo test` because it needs a browser; the file says how to run it.

## What is not done

See `.grill/parked.md`, section Milestone 6. A browser cannot read the certificate it is talking to, so the fingerprint is compared by eye; this is stated on the pairing screen. The relay, push notifications and phone apps belong to later milestones. Only English text.
