# Windsurf (→ Devin Desktop) — Feature Dossier
Registry: #92 | Category: 01 | Verified: unverified — closed-source IDE, no public repo; docs.windsurf.com now serves Cognition's rebranded "Devin Desktop" documentation (docs fetched 2026-08-23)

MAJOR FINDING first: Windsurf the editor is mid-rebrand to **Devin Desktop** under Cognition — official docs at docs.windsurf.com already ship as "Welcome to Devin Desktop", Linux packages install `devin-desktop` with `windsurf` kept as a transitional alias, and a "Legacy Windsurf Auth" path exists. Registry entry #92 should track Windsurf/Devin Desktop as one lineage. Closed source: internals stay unverifiable; everything below is doc-claimed surface only, cited from the official docs index.

## Feature inventory
- Rebrand/transition state: Devin Desktop naming, transitional `windsurf` package, VS Code/Cursor settings import, extension installs blocked from marketplaces (Open VSX recommendations instead) [VERIFIED-fetched:https://docs.windsurf.com/]
- Cascade agentic assistant: Code/Chat modes, tool calling, voice input, checkpoints, real-time awareness, linter integration [VERIFIED-fetched:https://docs.devin.ai/llms.txt (official index linked from docs.windsurf.com)]
- Devin Local agent harness: same harness as Devin CLI running on your machine as primary local agent inside the IDE [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Memories & Rules: auto-generated memories plus user-defined rules at global / workspace / system levels [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- MCP support over stdio/HTTP/SSE transports with admin controls [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Workflows: reusable markdown-file automations invoked via slash commands (PR review, deploy, test, format) [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Cascade Hooks: custom shell commands at pre/post points in agent workflow for logging, security controls, validation, governance [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Automatic git worktrees for parallel Cascade tasks [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Context Awareness RAG codebase indexing with `.devinignore`/`.codeiumignore` gitignore-style exclusions; Fast Context retrieval subagent (SWE-grep models, up to 20× faster code retrieval) [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Arena Mode: run multiple Cascade instances in parallel to explore different approaches simultaneously [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Terminal with Command mode, Turbo auto-execution, and allow/deny lists for command control [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Security Profiles restricting network, MCP, git, and GitHub-CLI access bound to orgs/automations/sessions; OS-level Sandbox with network filtering for CLI sessions; AI Guardrails screening prompt-injection/exfiltration; attribution filtering blocking public-repo code matches [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Adaptive model router automatically selecting best model per task [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- App Deploys: one-click deployment of built apps to Netlify/devinapps.com [VERIFIED-fetched:https://docs.devin.ai/llms.txt + https://docs.windsurf.com/]
- Devin Outposts: self-hosted workers on your own infrastructure watching a queue, claiming sessions, provisioning machines — control plane remains Cognition's [VERIFIED-fetched:https://docs.devin.ai/llms.txt]
- Editor conveniences: Tab completion, inline Command edits, Codemaps, Vibe and Replace, Previews, Quick Review, AI commit messages, DeepWiki symbol explanations [VERIFIED-fetched:https://docs.devin.ai/llms.txt]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Whole-tool adoption (Devin Desktop/Windsurf as LastHarness's coding surface) | ❌ Conflict-skip | — | Closed source, account login + credit metering required even for local-agent features, cloud-hosted models only documented; breaks self-hosted boundary, local-first inference, and auditability |
| Terminal allow/deny command lists + Security Profiles (network/MCP/git scoping) | 🧩 Mechanism only | Validates deny-by-default allow-lists §5/§8 | Independent convergence on exactly LastHarness's invariant; corroboration, nothing new to build |
| Cascade Hooks (pre/post lifecycle shell commands) | ⚙ Feature adoption | Worker supervision/kernel governance §16.6 | Lifecycle hookpoints are where logging, validation, and security enforcement belong — maps onto kernel-side worker wrapping feeding the decision log |
| Auto worktrees per parallel task | 🧩 Mechanism only | Already specified §16.1 worktree fan-out | Confirms the pattern at scale; no change |
| `.devinignore` context exclusions | ⚙ Feature adoption | §8.2-style gated repo access for workers | Deny-by-default file-scoping of what an indexer may ingest — same shape as gated memory handles |
| Fast Context specialized retrieval subagent | ⚙ Feature adoption | Information dept retrieval worker | Dedicated fast-retrieval specialist role matches LastHarness's department-worker decomposition; mechanism claim (20×) untested locally |
| Auto-generated Memories at scoped levels | 🧩 Mechanism only | Department-scoped gated memory §6 | Pattern (auto-promote session facts into scoped memory) fits; their cloud storage does not |
| Adaptive model router | 🧩 Mechanism only | Model packs + single-endpoint facade | Corroborates tier-per-task routing LastHarness already specs via packs/aliases |
| Arena parallel approaches | 🧩 Mechanism only | Development dispatch fan-out §16.1 | N-best exploration pattern; kernel queue can express it without the IDE |
| App Deploys to Netlify | ❌ Conflict-skip | — | Third-party cloud execution violates self-hosted Linux boundary |
| Devin Outposts self-hosted workers | ❌ Conflict-skip | Watch-list note | Self-hosted execution is right-shaped but the orchestrating control plane stays external/cloud — fails egress and audit invariants for LastHarness use |

## Notable absences
No source available to verify any mechanism behind the doc claims (checkpoints, memory promotion, indexing are all black boxes). No fully air-gapped mode documented: every path — including Devin Local and Outposts — authenticates to Cognition's cloud and meters credits. No local-model inference option documented (model menu is Cognition SWE-1.x/Claude/GPT). No export of Memories/index data, so no migration path off the platform. Enterprise policy objects (allowlists, guardrails) exist but their enforcement engine is unverifiable.
