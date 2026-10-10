# Oh My OpenAgent (OmO) — Feature Dossier
Registry: #44 | Category: 05 | Verified: ~68.2k stars, SUL-1.0 per badge (GitHub API NOASSERTION), last push 2026-08-22 (GitHub API 2026-08-23)

## Feature inventory
- `ultrawork` / `ulw` keyword mode: one word activates the full agent stack which "doesn't stop until done" [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Discipline-agent roster with per-model tuning: Sisyphus orchestrator, Hephaestus autonomous deep worker, Prometheus interview-mode strategic planner, Oracle (architecture/debugging), Librarian (docs/code search), Explore (fast grep), Multimodal Looker [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Category-based delegation: orchestrator picks a category (`visual-engineering`, `deep`, `quick`, `ultrabrain`) and the harness automatically maps category→model; custom categories supported [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Hashline hash-anchored edits: every line returned to the agent is tagged with a content hash (`11#VK| code`); edits reference tags and are rejected on hash mismatch before corruption; cited Grok Code Fast 1 success 6.7%→68.3% from the edit tool alone [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Team Mode v4.0 (opt-in): lead agent + up to 8 parallel members communicating through dedicated `team_*` tools (`team_create`, `team_send_message`, `team_task_create`, `team_status`), real-time tmux grid visualization; hosts `hyperplan` (5 hostile plan critics) and `security-research` (3 hunters + 2 PoC engineers) [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Ralph Loop / `/ulw-loop`: durable multi-goal orchestration with evidence audit backed by a `.omo/ulw-loop/` state directory [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Todo Enforcer: system yanks an idle agent back to its task until completion [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- IntentGate: analyzes true user intent before classification or action [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Skill-embedded MCPs: skills carry their own MCP servers that spin up on demand scoped to the task and go away when done ("context window stays clean") [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Built-in runtime-injected MCPs: Exa web search, Context7 official docs, Grep.app GitHub code search [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- LSP suite (`lsp_rename`, `lsp_goto_definition`, `lsp_find_references`, `lsp_diagnostics`), AST-Grep pattern search/rewrite across 25 languages, tmux interactive terminal integration [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- `/init-deep`: generates hierarchical AGENTS.md files throughout the project tree; rules injection loads AGENTS.md/`.omo/rules/**` into context at every prompt [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Comment Checker (blocks AI-slop comments); 54+ lifecycle hooks (61 with Team Mode), all individually disableable via `disabled_hooks`; model fallback chains; session recovery [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Claude Code compatibility layer (hooks/commands/skills/MCPs/plugins work unchanged) plus Light Edition for Codex CLI via `npx lazycodex-ai install` [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]
- Anonymous telemetry default-on: one event/day/machine with SHA256-hashed install ID to PostHog; opt-out via config or env flags [VERIFIED-fetched:https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Hashline content-hash anchored editing | 🧩 Mechanism only | Edit-tool § | Deterministic fix for stale-line edit corruption; harness-level, model-agnostic, cheaply portable |
| Category→model automatic delegation | ⚙ Feature adoption | Hardware-tiered model packs § | The right indirection: workers request a capability class, kernel resolves to the hardware-appropriate local pack |
| Skill-embedded on-demand MCP servers (spin-up→teardown) | 🧩 Mechanism only | Skills/plugins dept § | Scoped ephemeral services match LastHarness's no-standing-process posture and context hygiene |
| Prometheus interview-before-code planning | ⚙ Feature adoption | Planning gate § | Interview→plan→approve flow fits propose-then-execute worker protocol |
| ulw-loop durable goal state + evidence audit | 🧩 Mechanism only | Orchestrator § | File-backed loop state with evidence trails aligns with decision-log auditing |
| Team Mode parallel specialists via `team_*` tools | 🧩 Mechanism only | Orchestrator § | Adopt the parallel-specialist topology but route all inter-member traffic through the kernel — no peer inboxes |
| ultrawork discipline loop + Todo Enforcer | 🧩 Mechanism only | Worker supervision § | Completion-pressure supervision within a bounded session lifetime |
| `/init-deep` hierarchical context files | 🧩 Mechanism only | Onboarding § | Auto-generated tiered project context aids token efficiency and cold-start |
| Default-on PostHog telemetry | ❌ Conflict-skip | — | Violates default-deny egress / local-first unless inverted to opt-in |
| Built-in always-on cloud MCPs (Exa/Context7/Grep.app) | ❌ Conflict-skip (as-shipped) | — | Third-party cloud calls must traverse LastHarness's egress gateway with explicit allow-listing, not ship baked in |

## Notable absences
No sandboxing story — hooks and plugins execute directly on host with full user permissions; Codex Light edition even offers an "autonomous full-permissions" install path. No memory subsystem of its own (pairs with external memory projects). No immutable audit log beyond loop evidence files. SUL-1.0 license is restrictive/non-OSI. Orchestration presumes subscription cloud models (Anthropic/OpenAI/Kimi/GLM), not local inference; hardware tiering absent despite category routing.
