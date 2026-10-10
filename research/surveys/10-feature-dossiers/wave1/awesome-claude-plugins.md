# Awesome Claude Plugins — Feature Dossier
Registry: #35 | Category: 05 | Verified: ~1,899 stars, 574 forks (GitHub API 2026-08-23), README badge says MIT but GitHub API detects no LICENSE (license: null), last push 2026-07-26, default branch `master`

## Feature inventory
- Curated list of production-ready Claude Code plugins plus several first-party plugin folders checked into the repo (connect-apps, frontend-design, artifacts-builder, theme-factory, canvas-design, commit, create-pr, pr-review, changelog-generator, ship, code-review, test-writer-fixer, debugger, backend-architect, mcp-builder, perf, audit-project, documentation-generator, security-guidance, skill-bus…) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-claude-plugins/master/README.md]
- Documented standard plugin structure: `.claude-plugin/plugin.json` metadata + optional `skills/<name>/SKILL.md`, `commands/*.md` slash commands, `agents/*.md` subagents, `hooks/hooks.json` event hooks [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-claude-plugins/master/README.md]
- Install mechanics: clone repo and run `claude --plugin-dir ./<plugin>`; multiple plugins load in one invocation (`--plugin-dir ./commit --plugin-dir ./code-review …`) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-claude-plugins/master/README.md]
- Thematic taxonomy of the catalog: Integrations; Frontend & Design; Git & Version Control; Code Quality & Testing; Backend & Architecture; DevOps & Performance; Documentation & Security; Developer Productivity; Companion & Personality; Image & Video Generation [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-claude-plugins/master/README.md]
- `connect-apps` plugin: one Composio MCP Gateway endpoint exposing 1,000+ SaaS integrations (Gmail, Slack, GitHub, Notion) with vendor-hosted auth, team access controls, audit logs; setup via `/connect-apps:setup` + dashboard API key [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-claude-plugins/master/README.md]
- External-list entries covering niche capabilities: AgentLint (33-check repo lint for agent compatibility), security-sweep (OWASP Top 10 scanner), asqav (tamper-evident session receipt with content hashes), context-mode (sandboxed subprocess output summarization, 98% context savings), skill-bus (declarative wiring of skills into other skills), backlog (event-sourced cross-session task MCP) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-claude-plugins/master/README.md]
- Contributing rules: real use case, no duplication, template structure, tested [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-claude-plugins/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| plugin.json + commands/agents/skills/hooks folder contract | 🧩 Mechanism only | Skills/plugins § | Proven packaging schema for bundling prompt assets with declarative capability — informs LastHarness's plugin envelope |
| `hooks/hooks.json` event hooks inside a plugin | 🧩 Mechanism only | Kernel lifecycle § | Declarative event subscriptions routed through a host = kernel-mediated hook dispatch pattern |
| skill-bus: wire context/conditions/skills into invocations without modifying them | 🧩 Mechanism only | Orchestrator § | Composition-over-mutation keeps skills immutable and routing kernel-side |
| context-mode sandboxed-subprocess output digestion | 🧩 Mechanism only | Worker sandbox § | Local subprocess summarization preserves context window; execution stays on-machine |
| asqav tamper-evident session receipts (file hashes before/after, hashed command log) | 🧩 Mechanism only | Decision log § | Independent prior art for hash-chained operation evidence in an immutable log |
| connect-apps / Composio MCP Gateway (1000+ SaaS actions) | ❌ Conflict-skip | — | WHERE DOES EXECUTION HAPPEN? Vendor cloud executes tool calls behind a dashboard API key with standing accounts; violates self-hosted boundary and default-deny egress |
| MyVibe instant-deploy, nano-banana Gemini image-gen, taisly social publishing | ❌ Conflict-skip | — | All depend on external hosted services/cloud inference; conflict with local-first inference |

## Notable absences
No permission model beyond what Claude Code itself enforces — plugins don't declare scoped capabilities or egress needs. No offline/local alternative for its flagship integration story. No versioning/compatibility matrix between plugins, no automated testing harness despite contributing rules asking for tests.
