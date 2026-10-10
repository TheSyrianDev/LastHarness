# Devin AI — Feature Dossier
Registry: #77 | Category: 01 | Verified: closed product — no public repo (registry: no-repo/closed); official source docs.devin.ai (linked from devin.ai), fetched 2026-08-24

THIN-SOURCE — skip-confirmation dossier for a closed product. Recorded disposition: autonomous cloud SWE skipped; the litmus ("WHERE DOES EXECUTION HAPPEN?") fails at step one — Devin's sessions run in Cognition-managed cloud infrastructure. The interim-progress-digests idea is already design-noted ("interim progress digests during long dispatches (77 Devin)", 09-feature-extraction:70,195). Everything below is doc-claimed surface only; there is no code to verify against.

## Feature inventory
- **Autonomous cloud software engineer**: "Devin is the AI software engineer"; sessions run on Cognition's cloud machines with IDE/browser/shell session tools; enterprise deployment options are Enterprise Cloud and Customer Dedicated Deployment — both Cognition-operated [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- **Environment blueprints**: declarative environment definitions (repos, tools, dependencies, env vars, secrets, file attachments) that build snapshots; Devin can draft a blueprint from your repo for review [VERIFIED-index:https://docs.devin.ai/llms.txt]
- **Devin Outposts — self-hosted workers**: sessions can run "on your own infrastructure" via self-hosted workers watching a queue, claiming sessions, provisioning machines — with an orchestrator API/CLI and PKCE partner integrations; the control plane remains Cognition's [VERIFIED-index:https://docs.devin.ai/llms.txt]
- **Interim progress digests (the already-noted idea)**: session monitoring/interaction surfaces (session tools, side chat, notifications) through which owners observe long-running work in progress [VERIFIED-index:https://docs.devin.ai/llms.txt]
- **Automations & auto-triage**: event-driven sessions triggered from Slack, GitHub, Linear, schedules, webhooks; a persistent Devin monitors a Slack channel and auto-triages incoming bugs [VERIFIED-index:https://docs.devin.ai/llms.txt]
- **Testing evidence**: end-to-end testing with video recordings sent as proof; Computer Use for GUI verification [VERIFIED-index:https://docs.devin.ai/llms.txt]
- **Security Profiles + guardrails**: reusable profiles restricting network/MCP/git/GitHub-CLI access bound to orgs/automations/sessions; AI Guardrails screening prompt injection/exfiltration; attribution filtering against public-repo matches; OIDC short-lived cloud credentials [VERIFIED-index:https://docs.devin.ai/llms.txt]
- **ACU metering**: consumption billed in Agent Compute Units with org/per-user limits [VERIFIED-index:https://docs.devin.ai/llms.txt]
- **Devin CLI + handoff**: local CLI coding agent (with its own OS-level sandbox and network filtering per docs index) able to `/handoff` tasks to cloud Devin sessions from CLI, Claude Code, or Codex [VERIFIED-index:https://docs.devin.ai/llms.txt]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Autonomous cloud SWE core (sessions in Cognition cloud / dedicated deployments) | ❌ Conflict-skip CONFIRMED | — | Litmus fails at step one: execution happens on Cognition infrastructure. Doc-claimed controls (Security Profiles, guardrails) cannot change where compute runs. Recorded skip stands. |
| Interim progress digests during long dispatches | 🧩 Already-noted mechanism | R7 owner visibility (09-feature-extraction:70) | Confirmation only: LastHarness renders dispatch progress from §16.6 Output statuses/spans kernel-side; Devin validates owner demand for mid-flight summaries. Nothing new to extract at thin-source strength. |
| Devin Outposts self-hosted workers | ❌ Conflict-skip | — | Workers may be yours, but queue/control plane/billing stay Cognition's — external standing state plus foreign control plane; also violates §16.5 ephemeral-worker doctrine. |
| Automations/auto-triage via Slack + persistent channel monitor | ❌ Conflict-skip | §16.4 messenger exclusion | Inbound unmediated chat triggering agents is the excluded pattern family (Hermes/OpenWorker precedent); no post-V1 adapter changes the standing-monitor problem. |
| Video recordings as test proof | 🧩 Mechanism note | §12.2 decision-log evidence artifacts | Attachable proof-of-behavior artifacts strengthen Evidence fields; only valid if captured inside the boundary (LastHarness would record in the Linux env, never upload to a vendor). |
| Blueprint-declared environments (reviewed, snapshot-built) | 🧩 Pattern echo | Worktree/env staging §16.1 | Declarative pre-dispatch environment with human-reviewed definition mirrors LastHarness's staging discipline; LastHarness grounds it in worktrees + pinned toolchains rather than VM snapshots. |

## Notable absences
No repo, no license, no code-level verification possible — every mechanic above is vendor-documented and could change without notice (the Windsurf→Devin Desktop rebrand already proved docs-level volatility; see wave1/windsurf.md and 99-corrections #92). Pricing/metering details sit behind sales pages. The CLI sandbox page exists but was not fetched at this thin tier — if LastHarness ever wants the CLI's local-sandbox mechanics as §7.1 prior art, fetch docs.devin.ai/cli/sandbox.md then. Deliberately minimal extraction per recorded disposition: the entry's only living value is the progress-digest confirmation; everything else is competitive context.
