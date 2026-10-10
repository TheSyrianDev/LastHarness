# OpenCut — Feature Dossier
Registry: #22 | Category: 07 | Verified: 85,644 stars, MIT, pushed 2026-08-10 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: the editor is product surface; any render pipeline is deferred to §16.4. Skip **confirmed**: everything in the README describes an owner-facing creative tool (web/desktop/mobile editor), which LastHarness deliberately does not build — Personal Agent owns owner-facing UX, and media *generation* lives in §16.4 fast-follows on separate serving stacks, not in a bundled NLE. What survives from this entry are browser-based architecture notes only.

## Feature inventory
- Free and open source video editor for **web, desktop, and mobile**; MIT licensed [VERIFIED-fetched:https://raw.githubusercontent.com/OpenCut-app/OpenCut/main/README.md]
- Project is **being rewritten from the ground up**; the previous version moved to `opencut-app/opencut-classic` and opencut.app still serves the classic build while the rewrite runs at new.opencut.app [VERIFIED-fetched:https://raw.githubusercontent.com/OpenCut-app/OpenCut/main/README.md]
- Rewrite roadmap: an **Editor API**, first-class third-party plugins via a **plugin-first architecture**, one codebase for desktop/mobile/browser on a **Rust core**, an **MCP server for AI agents**, **headless mode** for automation and batch rendering, and a scripting tab inside the editor [VERIFIED-fetched:https://raw.githubusercontent.com/OpenCut-app/OpenCut/main/README.md]
- Monorepo layout with separate `apps/web`, `apps/api`, `apps/desktop` surfaces (`moon run web:dev` :5173, `api:dev` :8787) provisioned via moonrepo proto [VERIFIED-fetched:https://raw.githubusercontent.com/OpenCut-app/OpenCut/main/README.md]
- Outside contributions closed during the architecture-design phase; community routed to Discord/issues [VERIFIED-fetched:https://raw.githubusercontent.com/OpenCut-app/OpenCut/main/README.md]
- Sponsored by fal.ai (generative image/video/audio model hosting) — signals intended AI-generation integration direction, not shipped capability [VERIFIED-fetched:https://raw.githubusercontent.com/OpenCut-app/OpenCut/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Video-editor product surface (timeline editing across web/desktop/mobile) | ❌ Conflict-skip | — | Confirmed skip: this is precisely the creator-product surface LastHarness does not build; §16.4 covers generation pipelines, not editing UX. |
| Headless mode + batch rendering + Editor API (roadmap) | 🧩 Mechanism only | §16.4 fast-follow reference note | The headless/batch-render seam is the only architecturally relevant idea: render as an API callable without the UI — same shape as kernel-dispatched render workers producing artifacts on request. Roadmap-only today, nothing to adopt. |
| MCP server for AI agents (roadmap) | 🧩 Mechanism only | §4.4 kernel-mediated tool surface | If it ships, it enters LastHarness only as a deny-by-default kernel-registered tool; never wired agent-to-editor peer-to-peer. |
| Browser-based architecture: Rust core compiled once for web/desktop/mobile, thin web client against a local API service | 🧩 Mechanism only | Validates ONE-boundary posture | Editing state stays client-side in the browser with a localhost API companion — execution remains on-device, consistent with local-first instincts, but irrelevant to LastHarness since no editor ships. |
| Plugin-first architecture | 🧩 Mechanism only | Extension-surface design reference | Reinforces the pattern that capability extensions carry stable interfaces; LastHarness's equivalent is kernel-registered skills/tools with allow-lists. |

## Notable absences
The current README contains **no feature inventory of the actual editor** — no timeline/capability list, no docs site, no screenshots of function; the project is mid-rewrite with contributions closed, so there is nothing stable to extract beyond roadmap intent. No self-hosted render service documented yet (headless mode is announced, unspecified). No licensing questions (MIT), but also no quality gates, provenance, or audit concepts — all deferred implicitly to whoever builds on top.
