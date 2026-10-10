# Open ChatGPT Atlas — Feature Dossier
Registry: #34 | Category: 07 | Verified: 445 stars, no license detected (`license: null` on GitHub API), pushed 2026-07-21 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: §16.7 chose agent-browser with explicit rationale — confirm what Atlas does differently and why it still loses. **Confirmed.** What Atlas does differently: it is a consumer browser *product* — a Manifest V3 Chrome extension plus a standalone Electron fork of the browser with the assistant embedded as a sidebar chat on every tab, doing automation **directly from the extension with no backend**, and offering two interchangeable execution modes (visual computer-use vs hosted tool-routing). Why it still loses: every path puts execution outside the boundary — inference is cloud Gemini by hard requirement, integrations execute through Composio's hosted Tool Router, and the safety story is per-click confirmation dialogs rather than policy. It is the inverse of §16.7's design.

## Feature inventory
- Ships two form factors: a Chrome/Edge **Manifest V3 extension** and a standalone **Electron browser application** with built-in Atlas capabilities [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]
- **Tool Router Mode**: Composio's hosted intelligent tool routing for Gmail, Slack, GitHub, and "500+ integrations" [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]
- **Browser Tools Mode**: Gemini 2.5 Computer Use driving visual browser automation — screenshots, clicks, typing, scrolling, navigation from natural-language instructions [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]
- Sidebar chat interface (React) accessible from any tab [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]
- Direct browser automation: no backend component; all API calls made directly from the extension to providers [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]
- Visual feedback during automation: blue click indicators and element highlighting [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]
- Safety features: confirmation dialogs for sensitive actions (checkout, payment, etc.) [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]
- Configuration requirements: a Google API key for Gemini is **required** (Pro/Flash/Flash-Lite); Composio API key optional for Tool Router mode [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/open-chatgpt-atlas/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Extension/Electron-browser assistant product (sidebar chat in every tab) | ❌ Conflict-skip | — | Owner-facing browsing/chat product surface; Personal Agent owns any such UX, and §16.7's agent-browser is a headless capability, not a consumer browser. |
| Browser Tools Mode via Gemini 2.5 Computer Use | ❌ Conflict-skip | — | Litmus: inference executes on Google's cloud on every screenshot/click decision — mandatory cloud dependency violates local-first; §16.7 keeps visual browser control in-boundary against local models behind allow-lists. |
| Composio Tool Router across 500+ SaaS integrations | ❌ Conflict-skip | — | Actions execute through Composio's hosted router (third-party execution + credential custody outside the boundary); also redundant — wave-2 already carries the composio plugin/secure-openclaw family separately (#32/#33). |
| No-backend direct-from-extension provider calls | ❌ Conflict-skip | — | The absence of a mediating plane is the anti-pattern: exactly the peer-to-peer egress shape LastHarness's kernel-mediated routing and default-deny gateway exist to prevent. |
| Confirmation dialogs for sensitive actions | 🧩 Mechanism only | §13.x owner-approval gate UX reference | The intent (human approval before checkout/payment-class effects) matches LastHarness's gated actions, but dialog-level prompting has no policy, scope, or audit layer — recorded as UX precedent only. |
| Click-indicator/highlighting visual feedback | 🧩 Mechanism only | §16.7 operator-visibility nicety | Useful observability detail for any future in-boundary browser automation; cosmetic otherwise. |

## Notable absences
No license file detected by GitHub API (`license: null`) — legally unusable as an adoption base even if the skip were overturned. Last push 2026-07-21 with 445 stars: low activity for a fast-moving category. No local-model support anywhere in the docs fetched (Gemini key required). No audit log of automated actions, no session isolation/sandboxing of the automated browser context, no memory or task-history persistence, and no pairing/scoping concept for who may instruct the agent — trust begins and ends at per-action dialogs.
