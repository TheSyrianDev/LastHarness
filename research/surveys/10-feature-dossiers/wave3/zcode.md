# ZCode — Feature Dossier (THIN-SOURCE)
Registry: #67 | Category: 01 | Verified: closed-source, no repo (registry row `no-repo(closed)`); official site zcode.z.ai only — Electron app v3.8.1 installers, © 2026 (site fetched 2026-08-24)

Skip-confirmation dossier. Recorded disposition: role-to-alias model matching already specified. THIN-SOURCE: evidence base is the marketing site (hero demo + plan pricing); docs exist at /en/docs but none are pulled into the registry — extracted accordingly, not padded.

## Feature inventory
- **Vendor-paired harness positioning**: "Official Harness for GLM-5.3"; "GLM is tuned for ZCode, making agentic coding faster and steadier"; "Deep GLM-5.3 integration — Optimized for GLM-5.3 across reasoning, coding, and multi-agent collaboration" [VERIFIED-fetched:https://zcode.z.ai]
- **Goals for long-running tasks**: "Use Goals to manage complex work with continuous planning, execution, and verification" [VERIFIED-fetched:https://zcode.z.ai]
- **Chat-channel bot control**: "Start and steer ZCode from WeChat, Feishu, or Telegram so work can keep moving anywhere" [VERIFIED-fetched:https://zcode.z.ai]
- **Multi-agent claims**: "next-gen vibe coding for complex goals—with multiple agents and control from anywhere" [VERIFIED-fetched:https://zcode.z.ai]
- **Electron desktop app** v3.8.1: macOS (Apple Silicon/Intel), Windows (x64/ARM64), Linux x64/ARM64 .deb + AppImage marked Beta; task UI shows per-task progress checklists, terminal, git tools, commit flow [VERIFIED-fetched:https://zcode.z.ai]
- **Subscription plans**: GLM Coding Lite/Pro/Max ($12.6–$117.6/month, weekly credits) sold on z.ai as the model access layer [VERIFIED-fetched:https://zcode.z.ai]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Harness tuned/matched to one model family (GLM-5.3) | 🧩 Mechanism only | Validates existing role→llama-swap-alias matching (§16.6 input 3; §16.1 aliases) | The recorded skip confirmed: pairing a harness to a matched model is ZCode's whole value proposition; LastHarness already specs the generic form — each role's dispatch names an alias ("coding", "tool-calling") resolved by llama-swap, no vendor co-tuning required. |
| Goals with continuous planning/execution/verification | 🧩 Mechanism only | Stored goal state + scheduled re-check §14.6 | Same goal-state pattern Prime Agent ships and the registry adopted for Automation; validation evidence, not a new mechanism. |
| WeChat/Feishu/Telegram steering | ❌ Conflict-skip | — | Unmediated chat channels commanding agents = the Hermes-family conflict (§16.5 standing presence + §13.4 owner-initiated-only remote access); LastHarness has no inbox for third-party chat platforms by design. |
| Multi-agent collaboration claims | 🧩 Mechanism only | Kernel-mediated routing §4.4 (no detail extractable) | Marketing-level claim only under THIN-SOURCE; whatever the mechanism, LastHarness's shape requires all inter-agent traffic through the kernel regardless. |
| Whole-product adoption | ❌ Conflict-skip | — | Closed Electron harness + subscription cloud account in the inference path; violates §16.2 substrate lock, inspectability expectations, and local-first defaults simultaneously. |

## Notable absences
THIN-SOURCE held: no public repo, no license, no API/CLI documentation surfaced on the landing page, no security or data-handling statements beyond standard ToS/Privacy links. No hardware/local-model story (cloud plans are the only model access shown). No permission/approval UX described. No memory or audit story. Nothing here is extractable beyond the two mechanisms above without inventing from marketing copy.
