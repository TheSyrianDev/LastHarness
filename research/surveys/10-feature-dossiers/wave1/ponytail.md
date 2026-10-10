# Ponytail — Feature Dossier
Registry: #10 | Category: 05 | Verified: ~108.2k stars, MIT, last push 2026-08-07 (GitHub API 2026-08-23)

## Feature inventory
- 7-rung minimal-code decision ladder injected as always-on ruleset: (1) need to exist? YAGNI-skip → (2) already in codebase? reuse → (3) stdlib → (4) native platform feature → (5) installed dependency → (6) one line → (7) minimum that works; ladder runs after reading the touched code ("lazy about the solution, never about reading") [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]
- Hard safety carve-out: trust-boundary validation, data-loss handling, security, and accessibility are never on the minimization chopping block [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]
- Intensity modes `lite|full|ultra|off` via `/ponytail`; default set by `PONYTAIL_DEFAULT_MODE` env or `~/.config/ponytail/config.json` `defaultMode` (default full); startup text shows current mode [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]
- Six commands/skills: `/ponytail-review` (diff over-engineering review, returns a delete-list), `/ponytail-audit` (whole-repo audit), `/ponytail-debt` (harvest deferred `ponytail:` shortcuts into a ledger), `/ponytail-gain` (measured impact scoreboard), `/ponytail-help` [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]
- Two tiny lifecycle hooks: UserPromptSubmit activates mode + injects the ruleset every turn; PreToolUse(`task`) injects ruleset into spawned subagents, scoped by `PONYTAIL_SUBAGENT_MATCHER` regex against agent_type [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]
- Agentic benchmark vs no-skill baseline (headless Claude Code on FastAPI+React repo, 12 tasks, n=4): −54% LOC, −22% tokens, −20% cost, −27% time at 100% safety; terse-prose control and bare "YAGNI+one-liners" prompt don't replicate it (control rises on tokens/cost/time; prompt arm drops to 95% safety) [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]
- 20-agent portability matrix: plugin installs for Claude Code, Codex, GitHub Copilot CLI, pi, OpenCode, Gemini/Antigravity CLI, Qoder, Hermes, Devin CLI, OpenClaw (ClawHub), Grok Build; AGENTS.md/rules-file adapters for editor agents (Cursor, Windsurf, Cline, Kiro, Junie, Amp, Jules…) [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]
- Companion positioning vs caveman skill: caveman shrinks what the agent says, ponytail shrinks what it builds — explicitly non-overlapping halves [VERIFIED-fetched:https://raw.githubusercontent.com/DietrichGebert/ponytail/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| 7-rung YAGNI ladder w/ safety carve-out | ⚙ Feature adoption | Coding standards / policy pack § | One-page deterministic decision procedure that measurably cuts code volume without touching validation/security/accessibility — aligns with quality gates and cost control |
| Subagent ruleset injection w/ matcher scoping | 🧩 Mechanism only | Kernel hook points § | Per-agent-type context injection via regex on agent type is a clean kernel-side primitive |
| `/ponytail-review` diff delete-list pass | ⚙ Feature adoption | Review gate § | Cheap adversarial "what can we delete" pass before merge; pairs with code-review dept workflow |
| Debt ledger harvest (`/ponytail-debt`) | 🧩 Mechanism only | Task queue / decision-log § | Explicit ledger for deferred work beats silent accrual; format is trivially reimplementable |
| Mode intensity dial (lite/full/ultra/off) | 🧩 Mechanism only | Policy pack § | Trivial but useful strictness knob per department or session |
| Agentic benchmark methodology (git-diff-scored arms, n-replicated) | 🧩 Mechanism only | Eval harness § | Reusable measurement design for judging any coding-behavior skill |

## Notable absences
Pure instruction layer — no enforcement hooks block over-built code, so compliance is probabilistic. No machine-readable output for CI gates. Nothing on execution sandboxing, egress, memory, or onboarding. Ruleset is generic coding philosophy, not domain knowledge. Requires `node` on the non-interactive shell PATH for its two hooks.
