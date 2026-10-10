# OpenWorker — Feature Dossier
Registry: #60 | Category: 07 | Verified: 14,989 stars, MIT, pushed 2026-08-24 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: approval gates/parked asks/deliverable-returns already specified — each confirmed below with the spec section that already covers it. One new conflict family (inbound Slack triggering, OAuth broker cloud) documented as the extract-don't-adopt boundary.

## Feature inventory
- **Approval gates**: "Before anything consequential - sending a message, changing a calendar, running a command - it checks in and you approve or redirect"; "writes, sends, and shell commands are approval-gated" [VERIFIED-fetched:https://raw.githubusercontent.com/andrewyng/openworker/main/README.md]
- **Parked asks**: "Unattended runs park their asks in an inbox instead of acting on their own" [VERIFIED-fetched:https://raw.githubusercontent.com/andrewyng/openworker/main/README.md]
- **Deliverable returns**: "delivers **finished work**, not just chat: a polished document, a Slack reply with the numbers…"; documents/spreadsheets/reports/web pages "land as files"; "You get the finished deliverable, not a to-do list" [VERIFIED-fetched:https://raw.githubusercontent.com/andrewyng/openworker/main/README.md]
- **Inbound Slack trigger**: "@OpenWorker" mention opens a session on your desktop and the answer returns as a thread reply [VERIFIED-fetched:https://raw.githubusercontent.com/andrewyng/openworker/main/README.md]
- **Schedules/automations** for recurring work with full transcripts landing in-app [VERIFIED-fetched:https://raw.githubusercontent.com/andrewyng/openworker/main/README.md]
- **25+ connectors + any MCP tool with per-tool control**, plus terminal and local files; engine is a local Python agent server built on aisuite, supervised by a Tauri desktop shell; per-launch token (`X-OpenWorker-Token`) written user-only, desktop uses an in-memory launch token [VERIFIED-fetched:https://raw.githubusercontent.com/andrewyng/openworker/main/README.md]
- **Privacy posture**: local-first — agent loop, conversations, connector tokens, model keys in local secret store; "The only cloud piece is a small service that brokers OAuth handshakes for connectors", with manual credentials as the sign-in-free alternative; BYO model incl. fully local via Ollama [VERIFIED-fetched:https://raw.githubusercontent.com/andrewyng/openworker/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Approval gate before consequential actions | 🧩 Mechanism only | Confirms §7.2 owner override + §16.6 input 4 deny-by-default allow-lists | The recorded skip confirmed: LastHarness's workers can only do what their dispatch-time role config allows, with owner override above — OpenWorker ships the same gate as product UX. |
| Parked asks inbox for unattended runs | 🧩 Mechanism only | Confirms notification-queue/inbox pattern (§14.6 Automation queue; §16.6 queued-owner-delivery on return) | Identical semantics to LastHarness's queue-and-deliver-when-the-owner-returns pattern: unattended work escalates instead of acting. |
| Finished-deliverable returns (files, not to-do lists) | 🧩 Mechanism only | Confirms §16.6 Output contract (artifact diff + per-role result schemas) | Worker value measured by validated artifacts returned through the dispatch contract, not prose — exactly the schema-gated return LastHarness adopted. |
| Recurring automations with transcripts kept | 🧩 Mechanism only | Automation dept §14.6 + immutable decision log §12 | Scheduled recurring work against stored state is already specced; transcript retention maps to the append-only log, not a second store. |
| Local secret store + per-launch/in-memory tokens between shell and server | 🧩 Mechanism only | Transport hardening checklist §13.4 | Process-to-process auth via short-lived launch tokens never persisted by the supervising shell is a tidy ingress-hygiene precedent for kernel↔surface auth. |
| Inbound Slack @mention triggering work | ❌ Conflict-skip | — | Unmediated inbound channel commanding agents violates §13.4's owner-initiated-only remote access and kernel-mediated routing (§4.4); same family as Hermes' direct-channel conflict. |
| OAuth-broker cloud service for connector handshakes | ❌ Conflict-skip | — | A third-party cloud service inside the credential path, even handshake-only, breaks local-first credential custody (§14.5 vault) and default-deny egress (§13.4.2); manual-credential mode shows the compliant path but hosted connectors stay individually allow-listed gateway routes. |
| Whole-product adoption | ❌ Conflict-skip | — | End-user AI-coworker product on a foreign engine (aisuite layer, §16.2) — Personal Agent owns any such surface; mechanisms transfer, product does not. |

## Notable absences
No memory system beyond conversation/connector state — no learned-fact lifecycle, no scoping taxonomy. No audit log with decision rationale (transcripts are kept but not structured for "why"). No egress policy engine: connector access is per-tool toggles, not destination-level allow-listing. Windows builds unsigned at beta time (disclosed). No hardware/local-model management beyond pointing at Ollama.
