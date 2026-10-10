# Anthropic Cybersecurity Skills — Feature Dossier
Registry: #21 | Category: 05 | Verified: ~30,655 stars, 3,647 forks (GitHub API 2026-08-23), Apache-2.0, last push 2026-08-20 · community project explicitly NOT affiliated with Anthropic PBC

## Feature inventory
- 817 structured cybersecurity skills across 29 security domains (Cloud Security 66, Threat Hunting 58, Threat Intel 52 … Hardware/Firmware 4), all following the agentskills.io open standard [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- Six-framework cross-mapping carried in each skill's frontmatter/reference files: MITRE ATT&CK v19.1 (805 skills, 290 distinct techniques), NIST CSF 2.0 (804), MITRE D3FEND v1.4.0 (139), NIST AI RMF (97), MITRE F3 fraud framework v1.1 (94), MITRE ATLAS (93) — each skill maps only to frameworks relevant to its type [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- Progressive-disclosure token economy: YAML frontmatter costs ~30 tokens to scan for discovery; full workflow body is 500–2,000 tokens loaded on task match — agent can sweep all 817 skills in one pass [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- Uniform skill directory anatomy: `SKILL.md` (YAML frontmatter + Markdown body) + `references/` (standards.md framework mappings, workflows.md deep procedure) + `scripts/` helper scripts + `assets/` report templates [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- Standardized frontmatter schema: `name` (kebab-case 1–64 chars), keyword-rich `description`, `domain`, `subdomain`, `tags`, plus typed framework fields `atlas_techniques`, `d3fend_techniques`, `nist_ai_rmf`, `nist_csf`, `version`, `author`, `license` [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- Four-section Markdown body contract per skill: When to Use (trigger conditions) / Prerequisites / Workflow (step-by-step commands + decision points) / Verification (how to confirm success, e.g. map findings to ATT&CK T1003) [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- Install mechanics: `npx skills add mukul975/Anthropic-Cybersecurity-Skills` or git clone; zero-config loading on any agentskills.io-compatible platform (Claude Code, Copilot, Codex CLI, Cursor, Gemini CLI, LangChain/CrewAI/MCP agents) [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- ATT&CK Navigator layer file shipped in release assets for visual coverage mapping; versioned releases (v1.0.0 = 734 skills/26 domains, grown to 817 on main) [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]
- Includes offensive/dual-use content (red-team C2, phishing simulation, exploitation) behind an authorized-use-only warning; SECURITY.md responsible-disclosure process [VERIFIED-fetched:https://raw.githubusercontent.com/mukul975/Anthropic-Cybersecurity-Skills/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Frontmatter-scan / lazy-body progressive disclosure (~30-token stubs) | 🧩 Mechanism only | Skills/plugins § | Pure discovery-indexing pattern; scales skill libraries without context blowout; no runtime dependency |
| When-to-Use / Prerequisites / Workflow / Verification body contract | 🧩 Mechanism only | Skills/plugins § + Decision log § | The Verification section produces checkable evidence — feeds naturally into the immutable decision log |
| Typed framework-mapping frontmatter (ATT&CK/CSF/D3FEND IDs on every skill) | 🧩 Mechanism only | Skills/plugins § | Machine-readable capability classification generalizes: any LastHarness skill can carry compliance/risk tags |
| Defensive-domain subset (hardening, forensics, incident response, supply-chain/SBOM triage) | ⚙ Feature adoption | Security dept | Directly serves the self-hosted Linux boundary: host auditing, ransomware response, dependency triage recipes are reusable text-only playbooks |
| Offensive/red-team skills (C2, phishing simulation, exploitation) | ❌ Conflict-skip | — | Dual-use tooling conflicts with deny-by-default egress posture and auditable-operation invariant; zero value for a personal assistant OS |
| 29-domain taxonomy + per-domain counts | 🧩 Mechanism only | Skills/plugins § | Demonstrates department-style categorization of a large skill corpus |
| ATT&CK Navigator layer asset | ❌ Conflict-skip | — | Visualization artifact tied to release assets; no LastHarness function served |

## Notable absences
No runtime at all — pure knowledge base, so nothing enforces Prerequisites or runs Verification; that burden falls entirely on the host agent. No memory/state between investigations, no decision history, no egress controls for skills whose workflows call external threat-intel feeds (STIX/TAXII, MISP). No non-technical onboarding surface; assumes a developer-grade agent host. License/community provenance is solid but the maintainer is an individual — content accuracy is PR-reviewed, not authority-reviewed.
