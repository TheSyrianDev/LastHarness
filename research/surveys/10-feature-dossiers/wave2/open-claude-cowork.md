# Open Claude Cowork — Feature Dossier
Registry: #32 | Category: 07 | Verified: 4,329★, MIT, last push 2026-05-23 (repo-verify.txt 2026-08-24; README lives on `master` branch — `main` 404s)

Open-source desktop Cowork clone: Electron chat app over a Node/Express backend powered by Claude Agent SDK or Opencode, with 500+ Composio SaaS integrations; the same repo bundles Secure Clawdbot (≈ standalone #33), a messaging-platform assistant. §C conflict recorded: standing cowork shell + integration breadth.

## Feature inventory
- Desktop chat application (Electron + React, dark-themed) for macOS/Windows/Linux automating work "end-to-end across desktop and all your work apps" [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]
- Multi-provider agent backend: Claude Agent SDK or Opencode SDK, switchable per configuration [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]
- Node.js + Express backend server (port 3001) hosting the providers, with real-time token-by-token streaming over Server-Sent Events [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]
- Persistent sessions — context maintained across messages via session IDs [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]
- Tool visualization sidebar showing tool inputs/outputs as the agent works [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]
- Skills support via `SKILL.md` files in `.claude/skills/` with frontmatter descriptions [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]
- 500+ app integrations via Composio Tool Router — Gmail, Slack, GitHub, Google Calendar/Drive, and more — keyed to a hosted Composio API key [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]
- Bundled Secure Clawdbot: personal AI assistant on WhatsApp/Telegram/Signal/iMessage with persistent memory, browser automation (navigate/click/fill/screenshot), natural-language reminders + cron jobs, and the same 500+ integrations [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-claude-cowork/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Standing desktop cowork shell w/ always-available chat | ❌ Violates ephemeral-workers invariant (§16.5) | — | The shell hosts a resident Express process running agent loops on demand-but-alive infrastructure; LastHarness surfaces are owner-initiated dispatches over ephemeral SDK workers |
| Persistent sessions across messages | ❌ Violates no-worker-state / ephemeral dispatch (§16.5) | — | Cross-message context lives in a standing server; compliant form is session records in the decision log with kernel-rehydrated context |
| Dual provider (Claude Agent SDK or Opencode) | ❌ Violates SDK lock (§16.2) | — | Second vendor substrate is exactly what §16.2 excludes |
| Express backend hosting agent loops | ❌ Violates standing-process ban (§16.5); execution outside kernel mediation (§4.4) | — | WHERE does execution happen? In an unmediated app server — the kernel must own spawn/routing/teardown |
| SSE token streaming UI | ⚙ Capability owner console owns | R7 owner visibility surfaces | Streaming display of live dispatches is pure presentation over trace events |
| Tool visualization sidebar (inputs/outputs) | ⚙ Capability owner console owns | §16.6 Output 3 traces → console render | Fourth independent arrival at owner-visible tool telemetry; strengthens the R7 gap |
| SKILL.md skills folder | ⚙ Capability skills registry owns | §14.5 skills/procedural store | Ecosystem-standard format; enters through the vetted versioned-registration path, not a raw folder drop |
| 500+ Composio integrations | ❌ Violates default-deny egress (§13.4.2) AND no-cloud-execution (§16.1) | — | Tool-router calls execute on Composio's cloud with outbound-by-default account access; breadth only returns as individually allow-listed gateway connectors |
| Clawdbot messaging half | ❌ Violates messengers-out-of-V1 (§16.4) + resident gateway daemon (§16.5) | — | See #33 dossier; bundled here for completeness, cross-noted in 99-corrections |

## Compliant re-implementation sketch
- Rebuild the core value — one conversational surface over all work — as the Personal Agent owner console (§4.5): a local UI where the owner initiates tasks; every task becomes a kernel dispatch to an ephemeral Claude Agent SDK worker.
- Kill the standing Express host entirely: sessions are dispatch records appended to the immutable decision log (§12); "continue this chat" rehydrates context via a kernel-composed brief, not a resident process.
- Streaming + tool visualization ride §16.6 Output 3 span-per-call traces into the owner console — same data the audit trail already captures, rendered live.
- Integration breadth shrinks to the default-deny egress gateway: each SaaS target (Gmail, Slack, GitHub…) is an explicit owner-approved allow-list entry with credentials held in the §14.5 vault; no hosted tool-router executes calls on the owner's behalf.
- SKILL.md drops unchanged into the §14.5 registry behind harness validation + versioned registration (the wave-1 supply-chain intake lifecycle).
- Browser automation rides the existing §16.7 agent-browser capability; natural-language reminders compile to §14.6 scheduled re-checks.
- Messaging-channel half defers to the post-V1 messenger adapter shape (external threads → owner-initiated kernel tasks; replies stream back).
- Stay out: Electron standing shell, Opencode second provider, cloud-executed tool routing, always-loaded free-form MEMORY.md (replaced by gated dept-scoped recall).

## Notable absences
No sandboxing or isolation story anywhere — the agent runs with host authority inside the user's machine. No tool allow-listing beyond SDK defaults; no approval gates before sensitive actions documented. No memory scoping, staleness handling, or audit log. No egress control — one Composio key grants the whole 500-app surface at once. Documentation is thin (single short README; deeper Clawdbot docs live in a subfolder not fetched); no tests, no multi-user/auth story.
