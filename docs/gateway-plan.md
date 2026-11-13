# Messaging gateway plan (proposal, not decided)

Status: planning. Hermes source read in this pass: `NousResearch/hermes-agent` (shallow clone, scratchpad only), files `gateway/platforms/base.py`, `gateway/pairing.py`, `gateway/authz_mixin.py`, `gateway/run.py`. Verified facts below; anything not listed is still unverified.

## What Hermes does (verified in source)
- Adapter base class `BasePlatformAdapter` (`gateway/platforms/base.py:1805`). Abstract methods include `connect(is_reconnect)`, `disconnect()`, `send(chat_id, content, reply_to)`, `get_chat_info(chat_id)`. One adapter per platform.
- Authorization is per platform, from env: `<PLATFORM>_ALLOWED_USERS`, `<PLATFORM>_ALLOW_ALL_USERS`, group allow-list env (`gateway/authz_mixin.py:31-35`).
- Unknown senders get a DM pairing code, not silence (`gateway/pairing.py` docstring): 8-char code from a 32-char alphabet via `secrets`; 1 h expiry; max 3 pending per platform; 1 request per user per 10 min; lockout after 5 failed approvals; files chmod 0600; codes never logged. Owner approves in the CLI.
- Startup refuses an "open" DM or group policy unless an explicit allow-all opt-in is set (`gateway/run.py:2210`, `_own_policy_open_startup_violation`).
- Policy values per platform: `pair`, `ignore`, `decline`, `open` (`gateway/config.py:163`).

## What to copy, what to skip
- Copy: pairing codes with the same limits, owner-approved in our desktop or terminal app; default-deny; no open policy without an explicit opt-in; secrets in `lh-secrets`.
- Skip for now: multi-profile routing, shared-bot satellites, WhatsApp identity aliases, hosted rooms. They add scope without a user need yet.

## Open questions (need Omar)
- Which platform first. Telegram per `docs/steal-plan.md`. Omar must create the bot and provide the token; code does not ask for it until he says so.
- Whether a gateway chat may use write tools. Proposal: no. A gateway cannot answer approval prompts, so write tools stay off in gateway sessions.

## Adapter interface (proposal, from Hermes shape)
- `connect`, `disconnect`, `send(chat_id, text)`, `receive() -> Message { sender_id, chat_id, text }`, `verify(request) -> bool` for webhooks.
- One adapter per platform in `crates/lh-channels`.

## Next step
- Write the trait as a test-only sketch with a fake adapter; no network code until Omar approves the platform and token.
