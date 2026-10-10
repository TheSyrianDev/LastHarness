# Agency Agents (msitarzewski/agency-agents) — Feature Dossier
Registry: #3 | Category: 05 | Verified: 147,765 stars, MIT, last push 2026-08-06 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: curated subagent role rosters ≈ §14 tables + registry packs. **Confirmed** — this is a large personality-driven roster of markdown agent definitions (well over 200 files; the README itself warns OpenCode's runtime caps at ~119 registered agents and the installer warns when a selection exceeds it), organized into divisions. The roles duplicate what LastHarness's §14 department tables and registry role packs already specify, minus any execution machinery. Nothing here is adoptable code or mechanism beyond what the registry already carries. [VERIFIED-fetched:https://raw.githubusercontent.com/msitarzewski/agency-agents/main/README.md]

## Feature inventory
- Divisional roster: Engineering (~59), Design (10), Paid Media (7), Sales (10), Marketing (~37 incl. a large China-platform cluster), Product (5), Project Management (7), Testing (9), Security (12), Support (6), Spatial Computing (6), Specialized (~50: legal, HR, finance, healthcare, gov-compliance…), Finance (5), Game Development (cross-engine + Unity/Godot/Unreal sub-tables) [VERIFIED-fetched:https://raw.githubusercontent.com/msitarzewski/agency-agents/main/README.md]
- Uniform per-agent file anatomy: identity & personality traits, core mission & workflows, technical deliverables with code examples, success metrics & communication style — pure prompt text, no scripts [VERIFIED-fetched:https://raw.githubusercontent.com/msitarzewski/agency-agents/main/README.md]
- Distribution: `scripts/install.sh --tool <name>` across 14 hosts (Claude Code, Cursor, Codex, Gemini CLI, OpenCode, Copilot, OpenClaw, Aider, Windsurf, Kimi, Osaurus, Hermes, Antigravity, Vibe) via `convert.sh` generating per-tool integration files; division/agent-level selective install [VERIFIED-fetched:https://raw.githubusercontent.com/msitarzewski/agency-agents/main/README.md]
- Companion native desktop app (agencyagents.app; macOS/Linux/Windows, separate repo) that browses the roster and installs agents with auto-update [VERIFIED-fetched:https://raw.githubusercontent.com/msitarzewski/agency-agents/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Curated role rosters across 14 divisions | ❌ Conflict-skip | — | Confirmed skip ≈ §14 tables + registry packs: LastHarness already owns its role taxonomy; importing hundreds of personality-flavored duplicates would bloat registries without adding mechanism |
| Personality-driven agent framing (voice/communication style per role) | 🧩 Mechanism only | Worker prompt-schema reference | One transferable fragment: a role card can carry tone/deliverable/success-metric fields alongside its mandate — the schema idea, none of the specific personas |
| Multi-tool convert/install scripts | 🧩 Mechanism only | §14.5 intake lifecycle (compile/register step) | Same markdown compiled into N host-native formats previews the "register once, project to consumers" half of the skills intake lifecycle |
| Desktop installer app | ❌ Conflict-skip | — | Owner-facing GUI outside LastHarness's surface; closed packaging, no mechanism |

## Notable absences
No execution semantics at all — no permissions, scoping, memory, tool policy, or routing; activation is "hey Claude, activate X mode" free-text. Roles assert success metrics but define no measurement or verification. Massive functional overlap between divisions and with gstack/registry roles. Breadth over depth: several niche roles (OrgScript, GaussDB, French consulting market) exist purely as prompt personas. Consistent with the recorded disposition: extract nothing further, never pad.
