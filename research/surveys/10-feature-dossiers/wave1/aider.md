# Aider — Feature Dossier
Registry: #91 | Category: 01 | Verified: 48,404 stars, Apache-2.0, last push 2026-05-22 (GitHub API 2026-08-23)

Git-integrated terminal pair programmer. Its repo-map mechanism is ALREADY harvested into spec §14.2 (Addendum 2026-08-12: tree-sitter symbol-graph repo map injected into Development dispatch contexts) — noted below, not re-adjudicated. This dossier hunts the OTHER mechanisms.

## Feature inventory
- Repo map of the entire codebase for large-project orientation — **already harvested → spec §14.2 addendum; listed for completeness only** [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- Git-native edit discipline: automatically commits each change with sensible commit messages; diff/undo of AI changes via ordinary git tooling [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- Post-edit verification loop: auto-lints and runs tests after every aider edit, then attempts to fix problems the linters/test suites detect [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- Watch-mode editor integration: run aider alongside any IDE; comment in code and aider picks up and acts on it [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- Cloud AND local LLMs: connects to almost any LLM including local models (best with Claude/DeepSeek/o-series/GPT-4o per docs) [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- 100+ code languages supported [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- Images & web pages can be added to chat as visual/reference context [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- Voice-to-code: spoken feature/bug requests transcribed to the coding session [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- Copy/paste mode: works with any LLM via its web chat by streamlining context-out/edits-in through a browser [VERIFIED-fetched:https://raw.githubusercontent.com/Aider-AI/aider/main/README.md]
- LLM leaderboards (aider's own code-editing benchmarks) published as a first-party evaluation surface [REPORTED-unverified: linked from README nav (aider.chat/docs/leaderboards), page not fetched]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Auto-commit-per-edit with generated messages | ⚙ Feature adoption | Development dept worker contract §16.5 | Every accepted worker edit lands as an isolated, reviewable, revertible commit — the git-shaped twin of the immutable decision log; cheap undo for non-technical owner |
| Auto lint+test after each change, auto-fix loop | ⚙ Feature adoption | Worker validation gate (Development dispatch contract) | Bounded post-edit verify-and-repair before results return to manager; mirrors going-live gate thinking at per-change granularity |
| Repo map injection | 🧩 Mechanism only | Already spec'd §14.2 | Harvested 2026-08-12; no new verdict |
| Works-with-local-models connectivity | ⚙ Feature adoption | Local inference dept endpoint compatibility list | Any coding-agent pattern LastHarness keeps must stay OpenAI-compatible-endpoint friendly per local-first invariant |
| Watch-mode comment triggers | 🧩 Mechanism only | Onboarding/dispatch input patterns | Implies a standing file-watcher outside the kernel — keep as UX pattern (comment-as-instruction), implement via kernel dispatch instead |
| Voice-to-code | 🧩 Mechanism only | Non-technical onboarding § | Input modality worth remembering for owner-facing surfaces; no execution-path implications |
| Images/web pages as chat context | ⚙ Feature adoption | Vision pack consumers (Research dept) | Already-wired vision model pack gains a concrete use: screenshots/reference docs into worker context |
| Copy/paste-to-web-chat mode | ❌ Conflict-skip | — | Routes code through third-party web chats — violates local-first + default-deny egress |
| Whole-tool adoption (aider as LastHarness's coder) | ❌ Conflict-skip | — | Interactive TUI runtime ≠ ephemeral SDK worker under kernel routing (same ruling family as §16.2 framework-lock); extract mechanisms only |

## Notable absences
No headless/API/service mode documented in the README — it is a human-driven TUI, so embedding means wrapping, not importing. No permission allow-listing: aider assumes repo-write authority, nothing like per-tool deny rules. No persistent memory across sessions, no department/task scoping of context beyond the repo map. No egress controls (endpoint choice is config trust). No scheduling/batch mode — one conversation at a time.
