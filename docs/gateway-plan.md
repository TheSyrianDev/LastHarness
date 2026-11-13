# Messaging gateway plan (proposal, not decided)

Status: planning only. Written from the local surveys (`research/surveys/00-index.md` #53 Hermes Agent, #83 OpenClaw, #142 AstrBot). The Hermes gateway source was NOT read in this pass; the points below are to verify before any coding.

## What it would do
- Receive messages from a chat platform (Telegram first, per `docs/steal-plan.md`), send them to one agent session, and send the reply back.

## Open questions (need Omar)
- Which bot token and account: Omar must create the bot and provide the token. No token is stored or requested by code until he says so.
- Who may send to the agent: an allow list of platform user IDs, set by Omar. Default: nobody. Unknown senders get no reply.
- Whether a gateway chat may use write tools. Proposal: no. Write tools need the desktop or terminal approval path, which a gateway cannot answer.

## Adapter interface (proposal)
- `receive() -> Message { sender_id, text, chat_id }`, `send(chat_id, text)`, `verify(request) -> bool` for webhook signatures.
- One adapter per platform, in its own crate under `crates/lh-channels` (already exists).

## Security
- Secrets stay in `lh-secrets`. Never in the chat log.
- Rate limit per sender. Replies are plain text; no tool output is sent back without a size cap.
- Platform text is data, not instructions: it goes through the same prompt-injection rules as web pages.

## Next step
- Read the Hermes gateway docs and source (not done), then write the adapter trait as a test-only sketch.
