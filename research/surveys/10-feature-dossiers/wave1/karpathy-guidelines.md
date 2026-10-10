# Karpathy Guidelines (andrej-karpathy-skills) — Feature Dossier
Registry: #27 | Category: 05 | Verified: ~205,267 stars, 21,005 forks (GitHub API 2026-08-23), license: README states MIT but no LICENSE file detected by GitHub API (license: null), last push 2026-04-20

## Feature inventory
- Single `CLAUDE.md` behavior-patch file derived from Andrej Karpathy's observations on LLM coding pitfalls ("models make wrong assumptions… don't seek clarifications… overcomplicate code… change code they don't understand") [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]
- Principle 1 — Think Before Coding: state assumptions explicitly, present multiple interpretations instead of picking silently, push back when a simpler approach exists, stop and ask when confused [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]
- Principle 2 — Simplicity First: no features beyond what was asked, no single-use abstractions, no unrequested configurability, no error handling for impossible scenarios; litmus "would a senior engineer call this overcomplicated?" [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]
- Principle 3 — Surgical Changes: touch only what the task requires, match existing style, never delete pre-existing dead code, remove only orphans created by your own change; every changed line must trace to the request [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]
- Principle 4 — Goal-Driven Execution: transform imperative tasks into verifiable goals ("fix bug" → "write reproducing test then make it pass"); multi-step tasks declare step → verify pairs so the model can loop independently against success criteria [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]
- Two install paths: Claude Code plugin (`/plugin marketplace add forrestchang/andrej-karpathy-skills` + `/plugin install`) or curl-append into per-project `CLAUDE.md`; designed to merge with project-specific rules [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]
- Companion Cursor rule file `.cursor/rules/karpathy-guidelines.mdc` applying the same guidelines in Cursor [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]
- Self-check heuristics for whether the guidelines are working: fewer unnecessary diff lines, fewer rewrites from overcomplication, clarifying questions before implementation, minimal PRs [VERIFIED-fetched:https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Think Before Coding (surface assumptions/ambiguity before acting) | 🧩 Mechanism only | Kernel system-prompt § / Orchestrator § | Prompt-policy pattern that reduces silent wrong assumptions in worker prompts; pure text, kernel-mediated |
| Goal-Driven Execution (declarative success criteria + verify loops) | 🧩 Mechanism only | Orchestrator § + Decision log § | Step→verify pairs generate exactly the evidence trail an immutable decision log wants; enables autonomous loop-until-verified workers |
| Surgical Changes (traceability of every changed line to the request) | 🧩 Mechanism only | Worker sandbox § | Scope-containment rule for ephemeral SDK workers; complements allow-listed file access |
| Simplicity First anti-bloat rules | 🧩 Mechanism only | Kernel system-prompt § | Cheap prompt-level guardrail; keeps generated LastHarness features small |
| Plugin-marketplace vs CLAUDE.md dual packaging | 🧩 Mechanism only | Skills/plugins § | Shows one guideline artifact distributable both globally and per-project |

## Notable absences
No executable component whatsoever — nothing measures compliance with the four principles (the "how to know it's working" section is manual eyeballing). No enforcement hooks, no CI check, no decision log of when the agent pushed back. Repo is effectively dormant since April 2026 push. License file not detected via API despite MIT claim in README.
