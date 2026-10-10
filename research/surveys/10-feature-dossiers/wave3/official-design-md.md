# Official DESIGN.md (VoltAgent/official-design-md) — Feature Dossier
Registry: #26 | Category: 05 | Verified: 482 stars, MIT, last push 2026-07-26 (GitHub API 2026-08-24)

## Feature inventory
- Curated tracker index of first-party DESIGN.md files: every entry is "a DESIGN.md published by the company or project itself — not extracted, not reverse-engineered, not community-made"; current count badge: 7 (Atlassian, Clerk, Mintlify, Nuxt, Resend, Vercel, VoltAgent), each linking the canonical company-hosted URL (atlassian.design/DESIGN.md, clerk.com/design.md, vercel.com/design.md…) [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/official-design-md/main/README.md]
- Concept attribution and definition table: DESIGN.md introduced by Google Stitch ([getdesign.md](https://getdesign.md/what-is-design-md)) as a plain-text design-system document AI agents read for consistent UI; AGENTS.md ("how to build the project", read by coding agents) vs DESIGN.md ("how the project should look and feel", read by design agents) side-by-side [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/official-design-md/main/README.md]
- Tracking mechanics: entry = company name + link + one-line design characterization; drift handled by human reporting ("Report a change: let us know if an official file moved or was taken down"); contributions gated issue-first before PR; repo explicitly does not host linked content ("All linked files remain the property of their respective owners") [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/official-design-md/main/README.md]
- Provenance-tiered companion split: extracted-from-websites / community-made DESIGN.md files are deliberately routed to a separate sibling repo ([awesome-design-md](https://github.com/VoltAgent/awesome-design-md)), keeping this index first-party-only [VERIFIED-fetched:https://raw.githubusercontent.com/VoltAgent/official-design-md/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| First-party-only sourcing criterion with a separate tier for extracted/community material | ⚙ Feature adoption | §5.4 provenance-tiered sourcing criterion | A live precedent that provenance tiers work as *structural separation* (two indexes), not just labels; adopt directly into §5.4's tier ladder — company-published beats community-extracted for any design/UI intake |
| Link-canonical-don't-host catalog with change-reporting channel | 🧩 Mechanism only | §14.5 skills intake lifecycle — register step | Pointer-based registration keeps upstream authority and surfaces drift without mirroring liability; same mechanics fit LastHarness's skill/design-source registry entries |
| AGENTS.md ∥ DESIGN.md dual-document framing | 🧩 Mechanism only | UI dept design ingestion + kernel config § | Reinforces that behavioral config (how to build) and aesthetic config (how it looks) are separate artifacts with separate readers — maps to dept-scoped context loading |
| Issue-first curation gate before accepting entries | 🧩 Mechanism only | Intake review gate §14.5 | Tiny but real instance of discuss-then-register, i.e. provenance → scan → pin → register with a human gate |

## Notable absences
Corpus is tiny (7 entries) and adoption is early — useful as a criterion source, weak as a content source. No machine-readable index (no JSON/API), no per-entry verification timestamps or freshness checks, no license capture for linked files. Link-rot risk is acknowledged but mitigated only by manual reports (thin by design: a 3-file repo whose value is the policy, not the data). Extraction kept shallow accordingly.
