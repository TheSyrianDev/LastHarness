# Emil Kowalski Skills — Feature Dossier
Registry: #28 | Category: 05 | Verified: ~31,584 stars, 1,773 forks (GitHub API 2026-08-23), MIT, last push 2026-08-21

## Feature inventory
- Skill pack of 12 skills "for designers and engineers to build better user interfaces," distilling the author's animation/design experience from Vercel and Linear; installed via `npx skills@latest add emilkowalski/skills` [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]
- `emil-design-eng` — flagship skill: animation-plus-design judgment rules (correct easing choice, semi-transparent shadows over solid borders, compounding small-details taste) [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]
- `animate` — builds an animation from scratch, choosing curve, duration, and properties; `animate-expo` — same discipline for React Native/Expo: gestures, sheets, haptics, screen transitions, keeping motion off the JS thread [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]
- `review-animations` — strict rule-based review of existing animations; `improve-animations` — audits all animations in a codebase and emits prioritized, self-contained fix plans executable by any agent [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]
- `find-animation-opportunities` — searches UI for places that genuinely benefit from motion and explicitly says what NOT to animate; `animation-vocabulary` — precise motion terminology so prompts produce intended results [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]
- `pick-ui-library` — agent picks from an author-curated list of trusted libraries instead of hand-rolling components or installing abandoned packages [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]
- `prototype` — builds multiple versions of a UI piece and steps through them with a switcher; `apple-design` — Apple interface/motion principles distilled from WWDC talks, translated for web [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]
- `write-swift` (Swift 6 concurrency, value types, Swift Testing) and `ask-sonner` (setup/styling/recipes for the author's toast library) round out the pack [VERIFIED-fetched:https://raw.githubusercontent.com/emilkowalski/skills/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Animation build/review/improve skill triad (create → audit → prioritized plan) | ⚙ Feature adoption | UI dept | Complete motion-quality workflow owned by LastHarness's UI dept; plans are self-contained text any ephemeral worker can execute |
| find-animation-opportunities incl. explicit do-not-animate list | 🧩 Mechanism only | UI dept § | Negative-guidance pattern (deny-list for motion) mirrors deny-by-default thinking in design decisions |
| pick-ui-library curated trusted-dependency selection | 🧩 Mechanism only | Kernel allow-list § / Onboarding § | Vetted-package allow-list instead of arbitrary installs = dependency allow-listing; also protects non-technical users from abandoned packages |
| review-animations as strict rules-based critic | 🧩 Mechanism only | UI dept QA § | Deterministic-critic-over-generative-output pattern generalizes to any dept's output QA gate |
| prototype multi-version switcher | 🧩 Mechanism only | UI dept § | Variant-generation + human-pick loop fits approval-gated UI work |
| apple-design web-translated motion principles | 🧩 Mechanism only | UI dept § | Reusable aesthetic doctrine text; no platform conflict since translated for web |
| write-swift / ask-sonner single-platform or single-vendor skills | ❌ Conflict-skip | — | Swift targets non-Linux app development; Sonner is one npm lib — outside LastHarness's function surface |

## Notable absences
No accessibility/contrast/reduced-motion coverage beyond what taste rules imply; no token-file integration (works on imperative code, not DESIGN.md-style tokens); no CI or automated regression check for animations. Execution still assumes a dev-machine agent host — no onboarding path for non-technical operators.
