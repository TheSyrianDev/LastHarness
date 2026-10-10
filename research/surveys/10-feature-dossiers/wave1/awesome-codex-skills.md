# Awesome Codex Skills — Feature Dossier
Registry: #36 | Category: 05 | Verified: ~15,991 stars, 1,556 forks (GitHub API 2026-08-23), license not detected (GitHub API license: null; no badge in README), last push 2026-07-26, default branch `master`

## Feature inventory
- Curated list of practical Codex CLI skills in five categories: Development & Code Tools, Productivity & Collaboration, Communication & Writing, Data & Analysis, Meta & Utilities; mix of first-party skill folders and external-repo entries [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]
- Codex skill mechanics documented: skills live in `$CODEX_HOME/skills` (default `~/.codex/skills`); each folder needs `SKILL.md` with `name` + `description` YAML frontmatter; Codex auto-triggers on description match and loads the body only after it fires, keeping context lean; restart required to reload metadata [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]
- Standard skill layout convention: `SKILL.md` (required) + optional `scripts/` for deterministic steps, `references/` long-form docs loaded on demand, `assets/` templates; explicitly avoid extra README/changelog inside a skill folder to keep context lean [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]
- Python skill-installer: `install-skill-from-github.py --repo <owner/repo> --path <skill-path> --name <name>` fetches a skill into `$CODEX_HOME/skills/<name>` [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]
- Authoring guidance: make `description` exhaustive about trigger conditions, body focused on execution steps; progressive disclosure via references/; scripts for repeatable operations [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]
- Meta-skills shipped as folders: `template-skill`, `skill-installer`, `skill-creator` (guidance for building skills with progressive disclosure), `skill-share` (share skills across teammates) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]
- First-party catalog highlights: codebase-migrate (reviewable-batch refactors with CI verification), gh-fix-ci, pr-review-ci-fix, create-plan, meeting-notes-and-actions, support-ticket-triage, paperjsx (local PPTX/DOCX/XLSX/PDF generation via local MCP server — no API key, no network calls), spreadsheet-formula-helper, datadog-logs via Composio CLI [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]
- `connect` / `connect-apps` skills route real actions through the Composio CLI to 1000+ SaaS apps [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/awesome-codex-skills/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| $CODEX_HOME/skills drop-in directory + frontmatter-triggered lazy loading | 🧩 Mechanism only | Skills/plugins § | Confirms the folder-per-skill + description-routing pattern across a second major agent host — portability evidence for LastHarness's skill format |
| scripts/ vs references/ vs assets/ layout split | 🧩 Mechanism only | Skills/plugins § | Separation of deterministic code, on-demand docs, and templates is exactly LastHarness's skill-envelope shape |
| Exhaustive-description trigger authoring rule | 🧩 Mechanism only | Kernel routing § | Trigger precision is what makes kernel-mediated skill routing reliable instead of keyword-lucky |
| skill-installer from GitHub repo+path | 🧩 Mechanism only | Skills/plugins § | Fetch-and-install flow must be egress-gated but the pattern (curated source → local install → metadata reload) fits |
| paperjsx fully-local document generation (no API key, no network) | ⚙ Feature adoption | Docs/Reports dept | Local-first artifact generation aligned with all invariants — reference implementation for offline report output |
| codebase-migrate reviewable batches + CI verification | 🧩 Mechanism only | Orchestrator § | Batch-with-verification loop matches ephemeral-worker execution with decision-log checkpoints |
| connect/connect-apps Composio CLI SaaS actions | ❌ Conflict-skip | — | Third-party cloud mediation of tool calls + standing vendor accounts violate self-hosted boundary and default-deny egress |
| codex-sms-verification, helium-mcp live news/markets | ❌ Conflict-skip | — | Require external hosted services/MCP clouds; execution happens off-machine |

## Notable absences
No capability scoping per skill (any installed skill can do whatever its instructions say); no permission or allow-list story. No uninstall/update lifecycle beyond manual file ops. License undetected despite "awesome" curation — reuse rights unclear for bundled skill text.
