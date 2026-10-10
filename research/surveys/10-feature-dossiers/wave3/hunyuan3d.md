# Hunyuan3D — Feature Dossier
Registry: #133 | Category: 07 | Verified: Tencent-Hunyuan/Hunyuan3D-2 — 14,564★, license NOASSERTION (custom Tencent community license), pushed 2025-10-28; family repo WorldClaw pushed 2026-08-13

## Feature inventory
- Hunyuan3D-2: high-resolution 3D asset generation via large-scale diffusion models (shape + texture) [VERIFIED-fetched:https://api.github.com/repos/Tencent-Hunyuan/Hunyuan3D-2]
- Hunyuan3D-2.1: image → high-fidelity 3D assets with production-ready PBR materials [VERIFIED-fetched:https://api.github.com/repos/Tencent-Hunyuan/Hunyuan3D-2.1]
- Hunyuan3D-1.0: unified framework for text-to-3D AND image-to-3D generation [VERIFIED-fetched:https://api.github.com/repos/Tencent-Hunyuan/Hunyuan3D-1]
- Hunyuan3D-WorldClaw: agentic 3D open-world generation at scale — agent-driven scene composition, most recently active family member (2026-08) [VERIFIED-fetched:https://api.github.com/repos/Tencent-Hunyuan/Hunyuan3D-WorldClaw]
- Hunyuan3D-Part: part segmentation and generation pipeline [VERIFIED-fetched:https://api.github.com/repos/Tencent-Hunyuan/Hunyuan3D-Part]
- Open weights released per-family under the Tencent Hunyuan community license (self-hostable; terms differ from OSI licenses) [REPORTED-unverified — NOASSERTION on API; LICENSE text not fetched this pass]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Text/image-to-3D asset generation (self-hosted weights) | ⚙ Feature adoption | §16.4 media fast-follow family; Media Production parked case | Same deferred-modality shape as image/video-gen: separate serving stack, local weights available — strengthens the parked case's eventual trigger |
| PBR material pipeline (2.1) | 🧩 Mechanism only | §16.4 fast-follow quality bar | Production-material output is the differentiator vs toy mesh generators when the fast-follow opens |
| WorldClaw agentic open-world generation | 🧩 Mechanism only | §16.5-scoped children driving generation stages | Agentic orchestration of gen stages maps onto legal scoped-children pattern; watch-item, not V1 |
| Part segmentation pipeline | 🧩 Mechanism only | Asset post-processing for any future media dept | Niche; recorded |
| Community-license weight reuse | ❌ Conflict-skip (conditional) | — | License is custom/NOASSERTION — reuse requires legal review; never assume MIT-style freedom |

## Notable absences
No unified "one repo" for the family — capabilities are spread across five repos with independent release cadences. No LastHarness-relevant serving layer documented in the flagship README this pass (serving story lives in sibling repos). Vision-pass caption ("coolest AI model of the year") is engagement framing; registered on verifiable family merits only.
