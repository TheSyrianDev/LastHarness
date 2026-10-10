# Browser Skills — Feature Dossier
Registry: #39 | Category: 05 | Verified: ~0 stars (GitHub API 2026-08-23), MIT, last push 2026-05-07

## Feature inventory
- Curated collection of production-ready browser-automation Skills (research, community mining, competitor monitoring, login-required actions) following Anthropic's open Agent Skills standard — each skill is a folder with `SKILL.md` (YAML frontmatter + Markdown instructions) plus optional `scripts/`, `references/`, `EXAMPLES.md` [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]
- Progressive skill loading: agent sees only name+description (~100 tokens) at session start; full SKILL.md body (<5k tokens) loads only on task match; auxiliary files load on demand [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]
- Frontmatter conventions including `when_to_use` trigger phrases and `allowed-tools` pre-approval of tools a skill needs [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]
- Execution rides Composio's **cloud browser**: managed auth, anti-bot stealth, "no local Chrome required"; wired via Composio CLI (`composio link browser_tool`, `composio execute BROWSER_TOOL_CREATE_TASK`) behind a dashboard API key [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]
- Skill catalog: `github-tracker`, `reddit-researcher`, `hacker-news-digest`, `product-hunt-scout`, `glassdoor-analyzer`, `app-store-reviews`, `price-monitor`, `news-aggregator`, `company-research`, `linkedin-researcher`, `vc-research`, `job-tracker`, `job-applier`, `event-rsvp`, plus upstream browserbase `browser`/`fetch`/`search` [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]
- Built-in human-review pause points in action-taking skills: `job-applier` stops at the review screen before submit; `event-rsvp` refuses paid events without explicit confirmation [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]
- Skill composition into pipelines (e.g. reddit-researcher → company-research → linkedin-researcher; job-tracker → glassdoor-analyzer → job-applier) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]
- Cross-agent portability: same skill folder works across Claude Code, Claude.ai, Claude API, OpenAI Codex, Cursor, Gemini CLI, Antigravity, Windsurf [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/browser-skills/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| SKILL.md folder format + progressive ~100-token stub loading | 🧩 Mechanism only | Skills/plugins § | Pure packaging/lazy-load pattern; solves context bloat for hundreds of skills; no runtime dependency |
| `when_to_use` triggers + `allowed-tools` pre-approval frontmatter | 🧩 Mechanism only | Kernel allow-list § | Declarative capability scoping per skill maps directly onto deny-by-default allow-lists |
| Composio cloud browser execution (managed auth, anti-bot, API-key SaaS) | ❌ Conflict-skip | — | WHERE DOES EXECUTION HAPPEN? Off-machine, third-party cloud, standing vendor account; violates self-hosted boundary + local-first |
| Browser-skill catalog (research/monitor/mining recipes) | ⚙ Feature adoption | Search/Data dept + Ops dept | Task *recipes* are reusable locally; the value is the workflow text, not the cloud runtime |
| Human-review pause before irreversible submit (job-applier, event-rsvp) | 🧩 Mechanism only | Kernel approval gate § | Confirmation checkpoint before side-effectful actions generalizes to LastHarness's approval gate |
| Pipeline composition of skills | 🧩 Mechanism only | Orchestrator § | Chaining declarative skill outputs→inputs is kernel-mediated routing, not peer-to-peer |
| Cross-agent folder portability | 🧩 Mechanism only | Skills/plugins § | Confirms open-standard skill dirs survive agent swaps |

## Notable absences
No self-hosted/local browser execution path at all (cloud-only runtime). No egress controls, sandboxing, or audit log for skill scripts. No decision history or findings store — outputs land wherever the host agent puts them. No non-technical onboarding surface; assumes CLI + developer agent host.
