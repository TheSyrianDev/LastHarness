# Odysseus AI — Feature Dossier
Registry: #101 | Category: 07 | Verified: 86,120 stars, AGPL-3.0-or-later, pushed 2026-08-20, default branch `dev` (GitHub API 2026-08-24)

Skip dossier — competitive teardown-lite. Recorded disposition: its surfaces already exist as §4.5 / §14.5 / §14.6; the value of this file is a surface-set inventory against LastHarness's, not extraction. Odysseus is the same species as LastHarness (self-hosted AI workspace) aimed at consumers/prosumers, which makes it the wave's best mirror: same instincts (local models, one box), opposite governance (no kernel, no gates, AGPL monolith).

## Feature inventory (surface set)
- **Deployment shape**: docker-compose up from `.env.example`; web UI at localhost:7000; first admin password printed in container logs; native installs + GPU notes in docs/setup.md [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Chat + Agents**: local/API models with tools, MCP, files, shell, skills, and memory — the agent runtime is embedded directly in the workspace app [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Cookbook**: "hardware-aware model recommendations, downloads, and serving" — local-model lifecycle management inside the product [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Deep Research**: multi-step web research with source reading and report generation [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Compare**: blind side-by-side model testing and synthesis [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Documents**: writing-first editor with AI edits/suggestions; Markdown, HTML, CSV, syntax highlighting [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Email**: IMAP/SMTP inbox with triage, tags, summaries, reminders, reply drafts — an agent-worked mailbox as a first-class surface [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Notes, Tasks + Calendar**: reminders, todos, scheduled agent tasks, CalDAV sync [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Extras**: gallery/image editor, themes, uploads, web search, presets, sessions, 2FA [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **License posture**: AGPL-3.0-or-later with ACKNOWLEDGMENTS.md [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]
- **Security guidance**: keep auth enabled, keep private data out of Git, don't expose raw model/service ports publicly — advice-level, no enforcement architecture described [VERIFIED-fetched:https://raw.githubusercontent.com/odysseus-dev/odysseus/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Chat + Agents with tools/MCP/files/shell/memory | ❌ Skip (surface exists) | §14.5 intake surfaces + ephemeral workers | Same functional slot as LastHarness's kernel-mediated worker dispatch, but implemented as one embedded runtime with no dispatch contract, allow-lists, or decision log. Inventory-only. |
| Scheduled agent tasks + reminders/calendar | ❌ Skip (surface exists) | §14.6 Automation dept | Confirms owner demand for time-based agent work; LastHarness's version runs as sanctioned scheduled re-checks through queues, not an app-side scheduler. |
| Email triage/summaries/reply drafts over IMAP/SMTP | ❌ Skip (surface exists) | §14.5/§14.6 notification-queue analog | The strongest competitive signal in the set: owners want agent-worked mail. LastHarness covers the need via gated mail intake + notification queue without embedding a mailbox server. |
| Cookbook hardware-aware model download/serving | 🧩 Pattern echo | §16.1 local inference (llama-swap) | Validates local-first inference demand; LastHarness's alias-based routing needs no in-product model store, though onboarding-flow language ("what can this box run?") may borrow the framing for non-technical setup. |
| Compare — blind side-by-side testing | 🧩 Mechanism note | Role-alias evaluation / verify-before-promote §11.3 | Blind A/B synthesis is a usable evaluation UX for picking per-role aliases during calibration; execution stays local either way. |
| Deep Research multi-step reports | ❌ Skip (surface exists) | Research dept workflows | Already-spec'd capability class; Odysseus adds no mechanism detail at README depth. |
| Documents editor + Notes/Tasks + gallery | ❌ Skip | — | Consumer workspace shell outside LastHarness's kernel scope; relevant only as proof the non-technical-owner audience expects these surfaces eventually (post-V1 consideration, not V1). |
| Monolithic docker-compose workspace (app+models+mail+docs in one stack) | ❌ Conflict-skip | ONE boundary ≠ one process | LastHarness shares the single-box instinct but rejects the undifferentiated-monolith form: departments, deny-by-default gates, immutable logging, and egress policy require internal boundaries Odysseus does not have. |

## Notable absences
No governance layer whatsoever at primary-source level: no allow-lists, approval flow, audit/decision log, memory gating, or egress control — security posture is three sentences of deployment advice. No multi-user/RBAC story beyond an initial admin password. No plugin/API extensibility named in the README (MCP client support is listed, no server/exposure story). AGPL-3.0 means code reuse into LastHarness would carry license obligations — another reason this stays inventory-only. The teardown's conclusion: every surface has a §4.5/§14.5/§14.6 home already; the delta LastHarness should consciously decide LATER (post-V1) is whether consumer-shell surfaces (documents/mail/gallery) belong in its roadmap at all — Odysseus demonstrates they are what makes the category feel complete to non-technical owners.
