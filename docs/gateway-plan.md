# Messaging gateway plan (proposal, not decided)

Status: planning. Hermes source read in this pass: `NousResearch/hermes-agent` (shallow clone, scratchpad only), files `gateway/platforms/base.py`, `gateway/pairing.py`, `gateway/authz_mixin.py`, `gateway/run.py`. Verified facts below; anything not listed is still unverified. Re-checked 2026-11-13 against hermes-agent commit 7b1ed4ed (depth 1 clone).

## What Hermes does (verified in source)
- Adapter base class `BasePlatformAdapter` (`gateway/platforms/base.py:1805`). Methods defined at `connect(self, *, is_reconnect: bool = False)` (`base.py:2624`), `disconnect()` (`base.py:2630`), `send(chat_id, content, reply_to=None, ...)` (`base.py:2634`), `get_chat_info(chat_id)` (`base.py:4660`). Not confirmed: that all four are abstract (my grep for `@abstractmethod` found nothing near them) and that the code enforces one adapter per platform (not checked).
- Authorization is per platform, from env: `<PLATFORM>_ALLOWED_USERS`, `<PLATFORM>_ALLOW_ALL_USERS`, group allow-list env (`gateway/authz_mixin.py:31-35`). A global `GATEWAY_ALLOWED_USERS` / `GATEWAY_ALLOW_ALL_USERS` fallback also applies (`gateway/authz_mixin.py:788`, `:798`).
- Unknown senders get a DM pairing code by default, not silence (`gateway/config.py:670` default `unauthorized_dm_behavior = "pair"`). Limits (`gateway/pairing.py:29-36`): 8-char code from a 32-char alphabet via `secrets` (`pairing.py:519`); 1 h expiry (`CODE_TTL_SECONDS`); max 3 pending per platform; 1 request per user per 10 min; lockout 1 h after 5 failed approvals (`pairing.py:672-673`); data file written mode 0600 (`pairing.py:290`); codes stored hashed. Owner approves in the CLI (`pairing.py` module docstring). Codes never logged: stated in the docstring, not re-checked against every log call.
- Startup refuses an "open" DM or group policy unless an explicit allow-all opt-in is set (`gateway/run.py:2209` function `_own_policy_open_startup_violation`; check at `run.py:2227-2233`, opt-in via `GATEWAY_ALLOW_ALL_USERS` or the platform's allow-all env).
- Unauthorized-DM behaviour values: `pair`, `ignore`, `decline` (`gateway/config.py:165`; `decline` is deduped for 24 h, `pairing.py:37`). `open` is not in that set. It is a separate DM/group policy read from platform extras or env, default `pairing` (`gateway/run.py:2222-2225`).

## What to copy, what to skip
- Copy: pairing codes with the same limits, owner-approved in our desktop or terminal app; default-deny; no open policy without an explicit opt-in; secrets in `lh-secrets`.
- Skip for now: multi-profile routing, shared-bot satellites, WhatsApp identity aliases, hosted rooms. They add scope without a user need yet.

## Open questions (need Omar)
- Which platform first. Telegram per `docs/steal-plan.md`. Omar must create the bot and provide the token; code does not ask for it until he says so.
- Whether a gateway chat may use write tools. Proposal: no. A gateway cannot answer approval prompts, so write tools stay off in gateway sessions.

## Adapter interface (proposal; `receive` and `verify` are ours, not Hermes)
- `connect`, `disconnect`, `send(chat_id, text)`, `receive() -> Message { sender_id, chat_id, text }`, `verify(request) -> bool` for webhooks.
- One adapter per platform in `crates/lh-channels`.

## Next step
- Write the trait as a test-only sketch with a fake adapter; no network code until Omar approves the platform and token.
