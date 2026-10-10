# Awesome DESIGN.md — Feature Dossier
Registry: #25 | Category: 05 | Verified: ~109,703 stars, 12,517 forks (GitHub API 2026-08-23), MIT, last push 2026-07-31

## Feature inventory
- Curated collection of 73 ready-to-use `DESIGN.md` files (badge: DESIGN.md count 73) — plain-text design-system documents extracted from real websites, meant to be dropped into a project root so any AI coding agent generates visually consistent UI [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/README.md]
- Adopts the `DESIGN.md` concept from Google Stitch: a markdown file defining "how the project should look and feel," positioned as the design-agent counterpart to `AGENTS.md` ("how to build the project") — no Figma exports, no JSON schema, nothing to parse [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/README.md]
- Fixed 9-section document spec per file: Visual Theme & Atmosphere; Color Palette & Roles (semantic name + hex + functional role); Typography Rules (full hierarchy table); Component Stylings (buttons/cards/inputs/nav with states); Layout Principles; Depth & Elevation; Do's and Don'ts guardrails; Responsive Behavior; Agent Prompt Guide (quick color reference + ready-to-use prompts) [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/README.md]
- Per-site file bundle: `DESIGN.md` (what agents read) + `preview.html` / `preview-dark.html` visual catalogs of swatches, type scale, buttons, cards; hosted at getdesign.md with deep links [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/README.md]
- Category organization: AI & LLM Platforms, Developer Tools & IDEs, Backend/Database/DevOps, Productivity & SaaS, Design & Creative Tools, Fintech & Crypto, E-commerce, Media & Consumer Tech, Automotive, plus "Retro Web" nostalgia series [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/README.md]
- Usage model is two steps: copy a site's DESIGN.md into project root, tell your AI agent to use it [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/README.md]
- Contribution model for extending/correcting files (fix wrong colors, missing tokens) via issue-first PRs [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| DESIGN.md as plain-markdown design contract read by agents | ⚙ Feature adoption | UI dept | Exactly the artifact LastHarness's UI dept needs: one local markdown token source that makes any generated screen consistent; text-only, local-first, no tooling |
| The 9-section spec incl. semantic color roles + Do's/Don'ts + Agent Prompt Guide | 🧩 Mechanism only | UI dept § | Reusable template structure for LastHarness's own DESIGN.md regardless of whose brands inspire it |
| AGENTS.md ↔ DESIGN.md separation of build-instructions vs look-and-feel | 🧩 Mechanism only | Skills/plugins § | Confirms splitting agent-behavior docs from design docs — two config surfaces, one kernel |
| preview.html / preview-dark.html visual catalogs | 🧩 Mechanism only | UI dept § | Cheap self-verifying render check for a design-token file; optional |
| Curated third-party brand library (Stripe, Apple, Linear…) | ❌ Conflict-skip | — | Extracted brand identities of external companies; LastHarness needs its own house style, not brand clones; also copyright-adjacent "as is" content |

## Notable absences
No generation or validation tooling — it ships documents only; nothing checks that agent output actually conforms to the DESIGN.md. No dark-mode pairing guaranteed per entry in the spec itself, no theming variables beyond prose+hex, no versioned/tiered token sets (hardware-tiered rendering budgets are out of scope). No non-technical onboarding story beyond copy-paste.
