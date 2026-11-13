# Messaging gateway plan (proposal, not decided)

Status: planning. Hermes source read: `NousResearch/hermes-agent` (shallow clone in scratchpad, not run). Re-checked 2026-11-13 against hermes-agent commit 0e72e8a2 (depth 1). The earlier note named 7b1ed4ed; the line numbers below matched 0e72e8a2 when checked. Each claim is marked **[verified file:line]** or **[unverified]**.

## What Hermes does (verified in source)
- Adapter base class `BasePlatformAdapter` **[verified gateway/platforms/base.py:1805]**. Abstract methods `connect(self, *, is_reconnect: bool = False)` **[verified base.py:2624]**, `disconnect()` **[verified base.py:2630]**, `send(chat_id, content, reply_to=None, ...)` **[verified base.py:2634]**, `get_chat_info(chat_id)` **[verified base.py:4660]**. All four carry `@abstractmethod` (checked at base.py:2623, 2629, 2633, 4659). The earlier "not confirmed" note is resolved.
- One adapter per platform enforced by code: **[unverified]**.
- Authorization is per platform, from env: `<PLATFORM>_ALLOWED_USERS`, `<PLATFORM>_ALLOW_ALL_USERS`, group allow-list env **[verified gateway/authz_mixin.py:31-35]**. Global fallback `GATEWAY_ALLOWED_USERS` **[verified authz_mixin.py:788]** and `GATEWAY_ALLOW_ALL_USERS` **[verified authz_mixin.py:798]**. Default is deny **[verified authz_mixin.py:740]**.
- Unknown senders get a DM pairing code by default, not silence: default `unauthorized_dm_behavior = "pair"` **[verified gateway/config.py:670]**.
- Pairing limits **[verified gateway/pairing.py:29-36]**:
  - 8-char code from a 32-char alphabet via `secrets` **[verified pairing.py:29-30, 519]**.
  - 1 h expiry (`CODE_TTL_SECONDS = 3600`, pairing.py:32).
  - Max 3 pending per platform (pairing.py:35, 517).
  - 1 request per user per 10 min (pairing.py:33, 628).
  - Lockout 1 h after 5 failed approvals (pairing.py:34, 36, 672-673).
  - Data file written mode 0600 **[verified pairing.py:290]**.
  - Codes stored as salted SHA-256 hashes **[verified pairing.py:491-492, 522-523]**. The earlier note said "stored hashed"; this is now confirmed.
- Owner approves in the CLI **[verified pairing.py:1-8 module docstring]**.
- "Codes never logged": stated in the docstring (pairing.py:7); **[unverified]** against every log call.
- Startup refuses an open DM or group policy unless an allow-all opt-in is set **[verified gateway/run.py:2209, check at 2229]**. Opt-in is `GATEWAY_ALLOW_ALL_USERS` or the platform's allow-all env.
- Unauthorized-DM behaviour values `pair`, `ignore`, `decline` **[verified gateway/config.py:165]**. `decline` is deduped for 24 h **[verified gateway/pairing.py:37]**. `open` is not in that set (config.py:165). It is a separate DM/group policy read from platform extras or env, default `pairing` **[verified gateway/run.py:2223, 2225]**.

## What to copy, what to skip
- Copy: pairing codes with the same limits, owner-approved in our desktop or terminal app; default-deny; no open policy without an explicit opt-in; secrets in `lh-secrets`. Every limit above is verified, so the numbers are safe to reuse.
- Skip for now: multi-profile routing, shared-bot satellites, WhatsApp identity aliases, hosted rooms. They add scope without a user need yet.

## Open questions (need Omar)
- Which platform first. Telegram per `docs/steal-plan.md`. Omar must create the bot and provide the token; code does not ask for it until he says so.
- Whether a gateway chat may use write tools. Proposal: no. A gateway cannot answer approval prompts, so write tools stay off in gateway sessions.

## Adapter interface (proposal; `receive` and `verify` are ours, not Hermes)
- `connect`, `disconnect`, `send(chat_id, text)`, `receive() -> Message { sender_id, chat_id, text }`, `verify(request) -> bool` for webhooks.
- Whether Hermes has its own `receive` or `verify` method: **[unverified]**. Only the four abstract methods above were checked.
- One adapter per platform in `crates/lh-channels`.

## Next step
- Write the trait as a test-only sketch with a fake adapter; no network code until Omar approves the platform and token.
