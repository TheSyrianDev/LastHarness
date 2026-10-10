# Impeccable — Feature Dossier
Registry: #132 | Category: 05 | Verified: 62,386 stars, Apache-2.0, npm impeccable v3.6.0, pushed 2026-08-25 (GitHub API + npm, fetched 2026-08-25)

Design language for AI coding harnesses: one skill exposing 23 design commands, a deterministic anti-slop detector (59 rules), live browser iteration, and DESIGN.md-native context files. Started as an extension of Anthropic's frontend-design skill. **Already installed in this OpenCode environment** (`~/.config/opencode/skills/impeccable/`, skill v4.0.1) — existing dependency.

## Feature inventory
- Single `/impeccable <command> <target>` skill entry: 23 commands (craft, init, document, extract, shape, critique, audit, polish, bolder, quieter, distill, harden, onboard, animate, colorize, typeset, layout, delight, overdrive, clarify, adapt, optimize, live) [VERIFIED-fetched:https://raw.githubusercontent.com/pbakaus/impeccable/main/README.md]
- `/impeccable init` writes PRODUCT.md (audience, brand/product lane, voice, anti-references) + optional DESIGN.md in Google Stitch format — portable to every DESIGN.md-aware tool [VERIFIED-fetched:README]
- Deterministic detector: 59 rules catching AI-slop tells (Inter-everything, purple gradients, nested cards, gray-on-color text, bounce easing, side-tab borders) plus general quality issues; runs with NO LLM and NO API key; CLI (`npx impeccable detect`) with JSON output + exit codes for CI PR gates; Chrome extension overlay for any page [VERIFIED-fetched:README]
- Live Mode: browser iteration on running app — element picker injected into the page, visual variants written directly to source, page-level steering [VERIFIED-fetched:impeccable.style]
- "Worlds" system: 177 human-reviewed complete graphic systems ("challengers") dealt against the model's own derived direction; winning world is committed fully — anti-hedging mechanism against model default-taste convergence [VERIFIED-fetched:impeccable.style]
- Hook lifecycle: post-edit hook inspects each UI change and returns findings straight to the agent (correct-in-place loop); provider-native hook manifests for Claude Code/Cursor/Codex/GitHub Copilot/Grok Build [VERIFIED-fetched:README]
- Cross-harness distribution: installs into ~18 harness skill directories (.claude/.cursor/.codex/.opencode/.gemini/.grok/.pi/etc.), git-submodule vendoring path, plugin marketplace path (Claude Code), manual copy fallbacks [VERIFIED-fetched:repo tree]
- Detector config: `.impeccable/config.json` with ignoreRules/ignoreFiles/ignoreValues/designSystem toggles + inline `impeccable-disable` waivers scoped per file or line [VERIFIED-fetched:README]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| 23-command design vocabulary | ⚙ Feature adoption | Development dept §14.2 frontend roles; skills registry §14.5 | Precise intervention names (typeset/distill/polish) beat vague "make it better" prompts — direct fit for frontend_developer role config |
| Deterministic 59-rule detector (no LLM) | ⚙ Feature adoption | Evidence Service proposal #4 / QA-engineer dispatch tail | LLM-free deterministic gate = same pattern as adopted #93 Cline checkpoints; CI-capable exit codes match Tier-2 monitoring |
| PRODUCT.md brief-once context file | 🧩 Mechanism only | §16.6 dispatch inputs (role config slice) | Same "capture once, read everywhere" shape as DESIGN.md injection (#25, already adopted) |
| DESIGN.md Google-Stitch-format output | ⚙ Feature adoption | Brand-context injection (adopted #25) | Format-compatible with already-adopted brand injection — strengthens that adoption |
| Live browser iteration (element picker → source) | 🧩 Mechanism only | §16.7 computer/browser-use capability | Rides existing agent-browser capability; the variant-write-to-source flow is the novel part worth harvesting |
| Worlds challenger deck (human-reviewed aesthetic systems vs model defaults) | 🧩 Mechanism only | Evolution System §10.3 / new-work selection | Anti-convergence mechanism: externally-committed alternatives beat model self-taste — mirrors asymmetric-evidence auditing addendum |
| Post-edit hook returning findings to agent | 🧩 Mechanism only | §16.6 lifecycle-hook gates (already substrate) | Confirms hooks-as-enforcement-substrate finding E1 from wave 1 |
| Cross-harness skill distribution (~18 providers) | skip | — | Distribution mechanics, not a LastHarness concern |

## Notable absences
No backend/API design guidance (frontend-only by design). No dark-mode-specific rule cluster. No i18n-aware layout detection beyond harden's text overflow. Detector rules are heuristic (regex/AST-level) — no rendering-based visual regression. No team-collaboration or review workflow.

## Compliant re-implementation sketch
N/A — no invariant conflicts. Everything here is adoptable within LastHarness's architecture: commands become skill-pack content through the intake lifecycle, the detector becomes an Automation-dept deterministic gate, and the DESIGN.md/PRODUCT.md files are exactly the kind of project-scoped Information-store artifacts §4.6 already holds.
