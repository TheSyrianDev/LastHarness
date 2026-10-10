# BoltAI — Feature Dossier (THIN-SOURCE)
Registry: #55 | Category: 07 | Verified: closed product, no repo; official site only, © 2026, latest release BoltAI 2.15.0 (~2026-08-17)

Skip-confirmation dossier. Recorded disposition: chat front-end ≈ Personal Agent §4.5. **Confirmed** — BoltAI is a polished native macOS BYOK chat workspace: exactly the owner-facing surface LastHarness does not build, and a closed binary at that, so nothing here is adoptable code or mechanism. THIN-SOURCE applies: no repository, no architecture documentation beyond marketing pages and help-center articles; every claim below is vendor-stated and implementation is unverifiable.

## Feature inventory
- Native macOS app (SwiftUI/AppKit), macOS 13+, Intel & Apple Silicon; one-time purchase with one year of updates [VERIFIED-fetched:https://boltai.com]
- BYOK multi-provider client: 300+ models across Anthropic, OpenAI, Google Gemini, Mistral, Azure, Bedrock, xAI, Perplexity, plus **local models via Ollama and LM Studio** [VERIFIED-fetched:https://boltai.com]
- MCP support: `~/.boltai/mcp.json` server config, local and remote MCP servers, per-agent tool toggles [VERIFIED-fetched:https://boltai.com]
- Workspace features: reusable agents (custom instructions/knowledge/tools), projects, multi-thread forking/branching, prompt library, context profiles [VERIFIED-fetched:https://boltai.com]
- OS-integration extras: global-shortcut instant overlay, ShotSolve screenshot→answer, dictation & inline editing [VERIFIED-fetched:https://boltai.com]
- Multimodal input: PDFs, screenshots, code, UI captures via vision models [VERIFIED-fetched:https://boltai.com]
- Privacy posture (vendor-stated): chats stored locally in SQLite, API keys passphrase-encryptable, requests go directly app→provider, no training on content [VERIFIED-fetched:https://boltai.com]
- Optional Cloud Sync across Mac/iPhone/iPad (encrypted in transit/at rest); pauses to read-only export when the updates plan lapses [VERIFIED-fetched:https://boltai.com]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Native chat front-end with agents/projects/prompt library | ❌ Conflict-skip | — | Confirmed skip ≈ Personal Agent §4.5: LastHarness's Personal Agent owns owner-facing conversation UX; a proprietary Mac app can never be its implementation. |
| Closed-source binary | ❌ Conflict-skip | — | Litmus failure by construction: execution happens inside an unauditable binary; cannot be verified against the ONE Linux-env boundary (macOS-only) nor contributed to. |
| Ollama / LM Studio local-model support | 🧩 Mechanism only | Validates §16.1 llama-swap alias approach | Independent confirmation that mainstream clients treat local servers as ordinary OpenAI-compatible endpoints — the assumption alias routing already relies on. Nothing to adopt. |
| MCP client configuration (`~/.boltai/mcp.json`, per-agent tool toggles) | 🧩 Mechanism only | §4.4 kernel-mediated registration reference | Same MCP shape LastHarness standardizes on; the per-agent allow/deny toggle mirrors deny-by-default scoping but lives inside a closed app. |
| Optional Cloud Sync + mobile access | ❌ Conflict-skip | — | Conversation data replicated to vendor cloud; conflicts with local-first posture even as an opt-in, and irrelevant since the client itself is skipped. |

## Notable absences
No source, no license beyond EULA, no self-host or Linux build, no audit log or decision-history export beyond app-level chat search/export, no department/team memory model (knowledge stops at attached documents per agent), no on-prem deployment path, and no verifiable detail behind any privacy claim. Feature depth on the site is broad but shallow — consistent with THIN-SOURCE extraction above; nothing padded.
