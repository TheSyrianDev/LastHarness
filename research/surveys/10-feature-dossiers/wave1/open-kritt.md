# open·kritt — Feature Dossier
Registry: #73 | Category: 06 | Verified: ~1.9k stars, AGPL-3.0, last push 2026-08-21 (GitHub API 2026-08-23)

## Feature inventory
- Open-source self-hosted security/vulnerability-research platform: decomposes research into small well-defined tasks, runs them across AI agents in parallel, combines output into de-duplicated ranked findings [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Workflow builder: chain focused prompts into reusable security-research playbooks (visual workflow UI on localhost:5173) [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Scan execution via Codex or Claude Code CLIs as agent runtimes, against remote or local repositories and their dependencies [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Verification post-scripts: validate reported issues, build proofs of concept, produce reports [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Export bundles: ZIP with share-safe manifest containing canonical findings, structured data, post-processing output, reports, PoCs; complete vs clearly-marked partial exports; attacker-influenced report/PoC source kept as plain text [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Prioritization pipeline: custom severity rankers, consistent finding schema, automatic de-duplication [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Bring-your-own model access: Codex login or OpenAI / Anthropic / OpenRouter keys [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Headless CLI (`./kritt-headless`) for server/no-browser hosts: imports portable workflow/post-script/skill/ranker JSON, creates scans with backend validation, shows status/stages/failure reasons, exports bundles; never displays finding contents in terminal [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Isolation model: tool-enabled agents run as **root** inside disposable job containers with writable repo copies and **direct internet access** (to install tools, compile, test); published threat model advises dedicated Docker host/VM [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]
- Default ports bind to `127.0.0.1`; backend ships no application auth — "keep the stack private" guidance [VERIFIED-fetched:https://raw.githubusercontent.com/Kritt-ai/open-kritt/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Focused-task decomposition → parallel agent fan-out → merge | 🧩 Mechanism only | Orchestrator § | Divide-and-merge orchestration pattern is kernel-mediated routing done right; adopt the shape, not the stack |
| Workflow builder (chained prompt playbooks, portable JSON) | ⚙ Feature adoption | Ops dept workflow editor | Portable declarative workflows validated by a backend before run matches kernel allow-listing |
| Finding schema + auto de-dup + severity rankers | ⚙ Feature adoption | Security dept | Deterministic post-processing over noisy agent output generalizes to any multi-worker task |
| Verification post-scripts / PoC building before acceptance | ⚙ Feature adoption | Security dept verify stage | Claims must be proven before entering decision log — same gate philosophy |
| Share-safe export manifest, partial-export marking, plain-text attacker content | 🧩 Mechanism only | Export/reporting § | Treats all model-consumed content as untrusted input to reports — strong hygiene precedent |
| Disposable job containers per scan | 🧩 Mechanism only | Worker isolation § | Ephemeral workers = LastHarness invariant, but implementation diverges (see root/internet below) |
| Root containers with direct internet egress | ❌ Conflict-skip | — | Directly violates default-deny egress gateway and least privilege; execution environment unacceptable as-is |
| Codex/Claude Code as required agent runtimes | ❌ Conflict-skip | — | Third-party runtimes violate the no-third-party-runtime boundary and local-first inference |
| Cloud model providers (OpenAI/Anthropic/OpenRouter) only | ❌ Conflict-skip | — | No local-model path documented; conflicts with local-first inference invariant |
| Loopback-only binds + explicit threat model doc | ⚙ Feature adoption | Infra security baseline | Deny-by-default networking posture + shipped threat-model document are practices worth copying |

## Notable absences
No local inference support at all. No egress filtering for its own agents (needs full internet by design). No authentication layer. No immutable audit log of agent actions/findings decisions. AGPL-3.0 complicates embedding into a closed product — mechanism extraction only, not code reuse.
