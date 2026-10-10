# Secure OpenClaw — Feature Dossier
Registry: #33 | Category: 07 | Verified: 1,188★, MIT, last push 2026-07-25 (repo-verify.txt 2026-08-24; README lives on `master` branch — `main` 404s)

Personal 24×7 AI assistant running as a gateway on messaging platforms (WhatsApp/Telegram/Signal/iMessage) with Claude Agent SDK or Opencode behind it, persistent memory, cron reminders, tool approvals, and 500+ Composio integrations. Standalone sibling of the Secure Clawdbot bundled in #32 (cross-noted). §C conflict recorded: resident personal-assistant daemon on messaging channels.

## Feature inventory
- Gateway process bridging four messaging platforms: WhatsApp via Baileys (QR-link auth, session persisted to `auth_whatsapp/`), Telegram via bot token, Signal via signal-cli registration, iMessage via `imsg` CLI (macOS-only) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Dual AI providers: Claude Agent SDK (requires Claude Code installed; API-key auth on remote deploys) or Opencode (auto-detects/starts a local server on port 4096); switchable from CLI/config, `/model` mid-chat [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Per-platform sender allowlists: `allowedDMs`/`allowedGroups` lists or `['*']`, `respondToMentionsOnly` for groups; messages from unrecognized senders silently dropped [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Tool approvals in permission mode `default`: assistant announces "Claude wants to use Bash", chat reply Y/N resolves it; `AskUserQuestion` rendered as numbered options; approvals time out after 2 minutes [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Persistent memory tree at `~/secure-openclaw/`: MEMORY.md (long-term preferences/people/decisions), daily `YYYY-MM-DD.md` logs, topic notes; loaded at conversation start; written only when the user asks; `/memory list|search` commands [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Scheduling/reminders: natural-language one-shots ("remind me in 30 minutes") and cron expressions ("0 9 * * *"), persisted to `~/.secure-openclaw/cron-jobs.json`, executing while the gateway runs [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Composio app integrations (500+ apps) with first-use OAuth authorization links per app [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Remote deployment recipe: $6/mo DigitalOcean droplet, Docker compose build with volume-persisted WhatsApp auth + memory, WhatsApp QR page at `:4096/qr`, ufw port opening, swap-add guidance for 1 GB builds [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Message queue with `/queue` status; runner.js coordinates queued runs [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Slash-command surface: `/new`, `/reset`, `/status`, `/memory`, `/model`, `/queue`, `/stop`, `/help` [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Agent configuration: workspace `~/secure-openclaw`, `allowedTools: ['Read','Write','Edit','Bash','Glob','Grep']`, `maxTurns: 100` [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]
- Local terminal-chat mode plus interactive setup wizard (adapters, config display, connection test, provider switch) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/secure-openclaw/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Resident 24×7 gateway daemon answering messaging channels | ❌ Violates ephemeral-workers/no-standing-daemons (§16.5); messengers out of V1 (§16.4) | — | The recorded conflict: an always-on agent loop lives outside dispatch; WHERE does execution happen? In a standing process serving unmediated channels |
| Four platform adapters (Baileys/telegram-bot-api/signal-cli/imsg) | 🧩 Pattern only | Post-V1 messenger adapter shape (recorded) | Adapter mechanics are channel plumbing; valuable only once §16.4 reopens, and then as thin relay, not resident intelligence |
| Dual provider (Claude SDK / Opencode) | ❌ Violates SDK lock (§16.2) | — | Second vendor substrate excluded |
| Sender allowlists + silent-drop of unrecognized senders | 🧩 Pattern only | Identity gate for any future channel adapter (§13 chain) | Right instinct — only authenticated owner traffic may create work; lands kernel-side as owner identity check before task creation |
| Chat-reply Y/N tool approvals w/ 2-min timeout + numbered options | ⚙(no-Docker: compose packaging excluded per §16.1 no-third-party-runtimes; self-hosted Linux box only) Capability approval surface owns | Approval gates/parked asks (already specified) | Confirms spec'd gates from a consumer product; timeout-expiry detail worth keeping when the messenger adapter exists |
| On-request-only memory writing (never proactive) | ⚙ Capability Information dept owns | Gated memory writes §6.2 | Matches deny-by-default posture for memory growth |
| Whole MEMORY.md loaded at every conversation start | ❌ Violates dept-scoped gated memory (§6.2) | Compliant analog: bounded scoped brief | Unscoped full-file injection is the anti-pattern; context must be kernel-composed and retrieval-bounded |
| Cron jobs executing while gateway alive | ❌ Requires the standing process (§16.5) | Scheduled re-check §14.6 | Compliant path already specified: scheduled wake-ups that let the kernel spawn ephemeral workers |
| Composio 500+ integrations w/ OAuth links | ❌ Violates default-deny egress (§13.4.2); calls execute on Composio cloud (§16.1) | — | Outbound-by-default account fabric; each SaaS target returns only as an individually allow-listed gateway connector |
| Docker/VPS deployment recipe | ⚙ Capability deployment owns | Self-hosted Linux boundary §2.1/§16.1 | Self-hosted Linux is exactly the sanctioned boundary — but the workload it hosts must be ephemeral workers, not a resident bot |
| Message queue + runner coordinator | 🧩 Pattern only | Dispatch queueing §7.1 | Bounded concurrency queue already spec'd; queuing belongs to the kernel, not the bot |
| allowedTools allow-list + maxTurns cap | ⚙ Capability worker contract owns | Deny-by-default allow-lists + capped loops §10.2 | Independent validation of both locked mechanisms arriving in a hobbyist product |

## Compliant re-implementation sketch
- Target slot: post-V1 **messenger adapter** in the shape already recorded — a thin stateless relay process (infra tier, same class as llama-swap) that receives platform messages and converts each into an owner-initiated kernel task; replies stream back to the thread.
- The relay holds NO agent loop, NO memory, NO cron scheduler, NO queue logic — it authenticates and forwards; all execution stays inside kernel-spawned ephemeral Claude Agent SDK workers.
- Identity: sender allowlists become the kernel-side owner identity check (§13 chain) — unrecognized senders are dropped before any task is created, preserving the silent-drop posture.
- Conversation continuity comes from kernel-composed briefs (decision-log-derived, dept-scoped) attached at dispatch — never a resident session or whole-file MEMORY.md load.
- Approvals: chat replies Y/N resolve entries in the existing approval queue; keep the 2-minute expiry mapping to parked-ask timeout so an unanswered approval never blocks the floor.
- Reminders: "every day at 9am" compiles to a §14.6 scheduled re-check entry; firing wakes the kernel, which spawns a worker to compose and send — nothing ticks between wake-ups.
- Egress: Composio replaced by gateway connectors allow-listed per SaaS target; OAuth tokens live in the §14.5 vault, refreshed through the gateway.
- Stay out: V1 entirely (§16.4), resident daemon, Opencode provider, cloud-executed integrations.

## Notable absences
No audit log of decisions or tool calls — approvals happen in-chat and vanish. Memory has no scoping, staleness handling, decay, or encryption; daily files grow unbounded. Static global tool allow-list only — no per-task/per-scope variation, no injection screening of inbound message content before it reaches the agent. No rate limiting on inbound messages. No multi-owner story (one human's phone = the security boundary).
