# Google Skills — Feature Dossier
Registry: #108 | Category: 05 | Verified: Apache-2.0 (README License section); star count/last-push unverified; repo under active development per README

## Feature inventory
- Official Agent Skills collection for Google products/GCP, organized in categories: multi-product solution skills, AI/ML (Agent Platform), Infrastructure (GKE), databases/analytics, developer tools, management/observability, Well-Architected Framework pillars, security/identity, hosting, ads, analytics [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- `npx skills add google/skills` installer with interactive per-skill selection from the repo [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- Bundled harness plugins that pair Skills + MCP servers: Claude Code (`claude plugin marketplace add google/skills`), Codex plugin marketplace, Antigravity CLI (`agy plugin install <repo>/<plugin-path>`) [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- GCP "solution-architecture workflow" and named reference-solution skills (RAG on GKE+AlloyDB, n-tier serverless, agentic analytics lakehouse, GKE inference migration) [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- Vertex AI Agent Platform operational skill set: deploy, model registry, tuning, RAG engine, eval flywheel, prompt management, troubleshooting, plus a Skill Registry skill [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- Observability/troubleshooting wizard skills: Cloud Monitoring chart generation, ListTimeSeries request generator, logging query generation, SLO alert configuration setup wizard, GPU/TPU disruption runbooks [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- Six Well-Architected pillar skills (cost, ops, performance, reliability, security, sustainability) as review checklists for designs [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- `gcloud` CLI skill written specifically "for AI agents" plus Google Agents CLI onboarding [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]
- Index of sibling official skill repos (Android, Dart/Flutter, Firebase/Firestore, Genkit, Google Maps) under one governance umbrella [VERIFIED-fetched:https://raw.githubusercontent.com/google/skills/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| GCP/GKE/Agent Platform execution skills | ❌ Conflict-skip | — | Execution happens on third-party cloud; violates self-hosted Linux boundary |
| Skills + MCP server plugin bundles per harness | 🧩 Mechanism only | §skill-packaging | Packaging pattern (docs + matching MCP tools shipped together) is vendor-neutral |
| SLO alert configuration setup wizard | 🧩 Mechanism only | §ops/runbooks | Interview→config-artifact wizard pattern reusable for LastHarness's own alerting dept |
| Log/metric query generation skills | 🧩 Mechanism only | §ops/runbooks | Query-authoring-from-intent pattern applies to local observability stack |
| Well-Architected pillar checklist skills | 🧩 Mechanism only | §design-review | Structured design-review rubrics adaptable to self-hosted architecture reviews |
| `gcloud`-for-agents CLI skill | ❌ Conflict-skip | — | Drives egress to external cloud control planes; only the "CLI skill for agents" doc genre is imitable |
| Per-skill selective install | 🧩 Mechanism only | §skill-format | Matches install-only-what-you-need hygiene; avoids context bloat |
| Central index of satellite skill repos | 🧩 Mechanism only | §skill-registry | Registry-of-registries pattern for LastHarness's own skill catalog |

## Notable absences
No offline/self-hosted variant of any capability; nothing runs without GCP projects, billing, and network egress; no permission model beyond the harness's own; no content addressing/pinning of skill versions in the README.
