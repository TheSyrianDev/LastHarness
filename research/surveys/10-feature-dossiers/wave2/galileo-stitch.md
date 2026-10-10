# Galileo AI → Google Stitch (THIN-SOURCE) — Feature Dossier
Registry: #112 | Category: 07 | Verified: no repo — closed Google Labs product; official site https://stitch.withgoogle.com is JS-rendered ("Stitch - Design with AI" title only, no crawlable docs); no stars/license/push date

## Feature inventory
- Official site crawl yields title only — no server-rendered feature documentation exists at stitch.withgoogle.com or /docs [VERIFIED-fetched:https://stitch.withgoogle.com]
- Provenance: Galileo AI acquired by Google in May 2025, relaunched as Stitch inside Google Labs (announced Google I/O May 20, 2025); usegalileo.ai redirects and legacy accounts are gone [REPORTED-unverified] (press: https://recatools.com/ai-directory/galileo-ui/, https://www.guvi.in/blog/google-stitch-ai)
- Text/image/sketch prompt → high-fidelity UI screen generation powered by Gemini models [REPORTED-unverified] (press: https://www.index.dev/blog/google-stitch-ai-review-for-ui-designers)
- Two modes: Standard (Gemini Flash class, Figma export) vs Experimental (Gemini Pro class, higher fidelity, image input, HTML/CSS export) [REPORTED-unverified] (press: https://www.guvi.in/blog/google-stitch-ai)
- Paste-to-Figma export preserving layers and component structure for downstream editing [REPORTED-unverified] (press: https://recatools.com/ai-directory/galileo-ui/)
- Front-end code export claimed as HTML/CSS, Tailwind, React, Vue, Angular, Flutter, SwiftUI — conflicting with reports that code handoff routes via Firebase Studio rather than direct export [REPORTED-unverified] (press: https://aitrendtool.com/tools/stitch vs https://www.uxpin.com/studio/blog/google-stitch-ai-design-tool-updates-ui-ux/)
- Stitch 2.0 (March 2026): infinite AI-native canvas keeping each generation as a separate iteration, multi-screen generation with a design agent that reasons across a project's full screen set, voice-input critique, interactive prototypes, app-flow generation [REPORTED-unverified] (press: https://aitrendtool.com/tools/stitch, https://www.guvi.in/blog/google-stitch-ai)
- DESIGN.md portable design-system rules file + MCP server + SDK for connecting Stitch to coding tools [REPORTED-unverified] (press: https://aitrendtool.com/tools/stitch)
- Annotate: visual feedback on a screen that the AI implements [REPORTED-unverified] (press: https://www.uxpin.com/studio/blog/google-stitch-ai-design-tool-updates-ui-ux/)
- Free with credit caps — figures conflict across sources: ~400 design + 15 redesign credits/day resetting midnight UTC vs ~350 generations/mo standard + 200/mo experimental mode [REPORTED-unverified] (press: https://aitrendtool.com/tools/stitch vs https://recatools.com/ai-directory/galileo-ui/)
- No SLA, no enterprise support; reviewers flag Labs-experiment discontinuation risk and accessibility defects in output (low contrast, missing ARIA labels) [REPORTED-unverified] (press: https://recatools.com/ai-directory/galileo-ui/, https://aitrendtool.com/tools/stitch)

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Prompt→UI screen generation (Gemini, browser-based) | ❌ | — | Execution litmus: everything renders on Google's cloud behind a JS app. Third-party cloud execution of agent workloads violates the single-boundary invariant; no self-host path exists. |
| Paste-to-Figma export | ❌ | — | Depends on the cloud product end-to-end; also binds LastHarness output to a third-party SaaS surface. |
| Code export to front-end frameworks | ❌ | — | Cloud-generated code with conflicting/confused documentation about what actually ships; unauditable provenance for code entering the boundary. |
| DESIGN.md portable design-system rules file | 🧩 | §14.2 Development dispatch context | The one stealable idea: a versioned plain-text contract file that constrains any UI generator to the house design system — trivially adoptable against local models without Stitch itself. |
| MCP server + SDK bridge to coding tools | 🧩 | §4.4 kernel-mediated tool registration | Validates the pattern of exposing a design tool as an MCP tool surface — but under LastHarness it would be kernel-registered and deny-by-default, pointing at an in-boundary generator. |
| Multi-screen design agent reasoning across a project's screens | 🧩 | §14.2 project-scoped context pattern | Project-level context (all screens as one coherent set, not one prompt at a time) is the transferable mechanism for any local UI-generation capability. |
| Voice-input critique loop | ❌ | — | Cloud speech path; LastHarness voice intake (if ever) is local STT into the approval gate, not a Labs product feature. |
| Free-tier credit metering | ❌ | — | Server-side quota on a closed product; nothing ownable, plus discontinuation risk makes it non-plan-worthy. |

## Notable absences
No repository, no license, no release history — every capability claim above rests on third-party press/reviews, none verifiable from primary docs; no self-hosted or offline mode; no data-residency or training-use statement for uploaded sketches/screens; no API terms for the SDK/MCP surface; no SLA or support commitment; no export format guarantee (sources contradict each other); accessibility failures reported in generated output.
