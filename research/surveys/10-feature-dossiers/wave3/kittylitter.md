# KittyLitter (litter) — Feature Dossier (THIN-SOURCE)
Registry: #71 | Category: 01 | Verified: registry row has NO repo ("-"); official site kittylitter.app. The site links GitHub dnakov/litter, which resolves to 0xSero/litter: 2,484 stars, GPLv3 + App Store/Play distribution exception ("Other" per API), pushed 2026-08-20 (site + GitHub API + raw README, fetched 2026-08-24) — a registry-correction candidate NOT applied here (out of write scope)

Skip-confirmation dossier. Recorded disposition: note-tier "mobile owner control plane, owner-initiated only", read as the phone companion to LocalStudio (#70) per the 99-corrections adjudication; wave-3 treats it as skip-confirmation of that companion read. THIN-SOURCE: feature documentation is the marketing site; the repo README is developer-facing only. Task focus: detail remote-driving mechanics as R7/§13.4 evidence.

## Feature inventory
- **Phone companion positioning**: native iOS + Android client for Codex, Claude, OpenCode, Pi, and Droid — switch agents per session or run several in parallel; LocalStudio.ai's own site pairs with it as its phone client (#70/#71 companion family confirmed) [VERIFIED-fetched:https://kittylitter.app]
- **Three connection modes**: LAN auto-discovery ("Finds agents running on your local automatically. No IP addresses, no manual configuration"), SSH ("Litter starts your agent on the server for you"), and alleycat P2P QR pairing through NATs/firewalls with "no VPN, no public IP, no port forwarding" [VERIFIED-fetched:https://kittylitter.app]
- **Session continuity**: "Pick up exactly where you left off. Sessions, threads, and context follow you everywhere" [VERIFIED-fetched:https://kittylitter.app]
- **Shared Rust core architecture**: single `codex-mobile-client` crate behind UniFFI bindings for iOS/Android; session state, streaming, hydration, reconnect, SSH, and auth all live in Rust, platform code stays thin (UI/permissions/notifications); `codex-slingshot` HTTP/stream transport; Ghostty submodule as terminal renderer [VERIFIED-fetched:https://raw.githubusercontent.com/dnakov/litter/main/README.md]
- **Voice input**: real-time voice with the agent while away from the desk; Apple Watch app target exists in the build system [VERIFIED-fetched:https://kittylitter.app + https://raw.githubusercontent.com/dnakov/litter/main/README.md]
- License: GPL-3.0 with additional permission for App Store/Play distribution [VERIFIED-fetched:https://raw.githubusercontent.com/dnakov/litter/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Phone-companion-to-desktop pairing (#70/#71 family) | 🧩 Mechanism only | Confirms R7 owner-console client/server split | Companion read confirmed from both ends: LocalStudio advertises KittyLitter pairing, KittyLitter drives the same agent set. The dumb-phone/smart-home-server split is exactly the shape R7's console proposal adopts. |
| Remote-driving mechanics: session hydration/reconnect, stream transport, remote agent start over SSH | 🧩 Mechanism only | R7/§13.4 evidence — concrete prior art for the mobile monitor-and-approve surface | This is the payload the task asked to detail: reconnect-and-hydrate session state, HTTP/stream transport, and start-the-agent-on-the-host are the exact mechanics a §13.4-tunnelled owner console needs; they ride tailscale/wireguard/SSH unchanged. |
| LAN auto-discovery of running agents | 🧩 Mechanism only | Owner-console discovery pattern §13.4 | Zero-config local-network discovery is sound inside the boundary; LastHarness's kernel already knows what it runs, so this remains evidence rather than adoption. |
| Alleycat NAT-traversal P2P pairing (QR, no VPN/port-forward) | ❌ Conflict-skip | — | Exceeds §13.4's connectivity list (encrypted tunnels: tailscale/wireguard/SSH; owner-initiated; logged). Ad-hoc hole-punching creates an unlisted ingress channel — the one mechanic that must NOT transfer even though its UX (QR bootstrap) is attractive. |
| Driving foreign harness CLIs (Codex/Claude/OpenCode/Pi/Droid) | ❌ Conflict-skip | — | Same ruling as t3-code/herdr: controlled substrates are wrapped vendor CLIs, excluded under §16.2 regardless of client quality; LastHarness's workers are Claude Agent SDK subagents. |
| Voice input + Watch surface | 🧩 Mechanism only | Owner-console input modality note (presentation-layer only) | Input modalities render from sanctioned dispatch surfaces once the console exists; no new authority implied. |

> SKEPTIC FIX: canonical repo slug `dnakov/litter → 0xSero/litter` resolved via GitHub API redirect on 2026-08-24 (API returns final location); citations above use the original dnakov path.

## Notable absences
THIN-SOURCE held: no documented auth model beyond QR pairing (no token/session/revocation ladder at README level — t3-code's dossier remains the richer auth-ladder source), no audit logging of remote actions (§13.4 requires IP/duration/action logs), no approval-gate UX described (observe + steer, not approve), no egress story. Repo README is build-focused; ARCHITECTURE.md ownership boundaries not pulled (out of thin-source scope). Registry correction pending: #71 recorded as no-repo while an active GPL repo exists.
