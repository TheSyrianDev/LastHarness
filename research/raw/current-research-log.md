# Tool Research — Human Summary & Status

> **The registry itself lives in [`00-index.md`](00-index.md)** — 130 entries registered over the research period; after the 2026-08-24 integrity sweep, **127 active** (+2 merges, +2 delists, all recoverable via ledger). Every active entry has a deep feature dossier. This file is the human-facing changelog.

## Instagram harvest provenance

Saved Instagram/X collections were harvested into `research/raw/harvest-instagram-2026-08-22.md` (+ `harvest-x-2026-08-22.md`), video-deep-passed where captions were engagement-bait, and registered as **#110–#131** across three verification passes (waves logged in `../99-corrections.md`).

## Wave 2+3 dossier coverage

- **Full registry coverage achieved**: 130 entries registered over the harvest period; after this integrity sweep **127 remain active** — and every one of them has a deep feature dossier (**127 dossiers** across waves 1–3 in [`10-feature-dossiers/`](10-feature-dossiers/): wave 1 = 40 adopt/slot-notes · wave 2 = 36 conflict-teardowns/harvest notes · wave 3 = 51 full-coverage confirmations).
- **New recurring gaps R11–R16** appended to [`11-department-blueprints.md`](../11-department-blueprints.md): kernel-composed handoff packets (§16.6), evidence-ledger append-policy (§12), incident-rule DSL (§14.7), knowledge-freshness governance (§14.5 intake), hardware-capability probe (§17.3 Below-Low), single-artifact packaging contract.
- **Proposal upgrades**: #1 Owner Console now has transport/auth/approval-UX evidence (t3-code pairing-token ladder, opentag resumable approval cards, herdr status triad); #4 Evidence Service absorbed comp-crm's evidence-ledger pricing ("blank beats confidently-wrong") as the §12 append-policy.
- **Parked department cases**: **Meeting-Ops** (Meetily proves invariant-clean local capture→summary; awaiting owner demand) and **Media Production** (ViMax/Stitch prove demand; blocked until §16.4 local image/video generators exist).

## Registry integrity sweep (2026-08-24)

| Decision | Entries | Reason |
|---|---|---|
| Merged | #38 → #37 | Composio Open Agents dissolved into stale fork duplicate of Vercel Open Agents |
| Delisted | #115 Handle | Unidentifiable after exhaustive search |
| Delisted | #129 AgentReach | Claimed ~68k★ product refuted; fallback repo stale |
| Fixed | #70, #71 | Repos found post-registration (statuses were stale) |
| Resolved | #53 | Star-inflation flag removed — ~235k★ API-verified genuine |
| Corrected | #111 | ViMax stars ~9.9k → ~12.1k |
| Flagged | #8, #98 | License-null pair — verbatim adoption blocked |

Full trail: [`99-corrections.md`](../99-corrections.md) · run journals under `runs/ulw-research/`.

## Wave 6 — Codebuff + Freebuff (#147, 2026-08-30)

- **Intake**: `CodebuffAI/codebuff` is a 301 redirect to `CodebuffAI/freebuff` — one canonical repo, 11,232★ / 1,204 forks / Apache-2.0 / pushed 2026-08-30 / active daily. SDK `@codebuff/sdk` v0.10.7 verified on npm (Apache-2.0, GitHub-OIDC trusted publisher).
- **Lineage decision**: registered as **one entry** mirroring Windsurf→Devin Desktop (#92). Codebuff = open framework + SDK + 150-agent marketplace. Freebuff = ad-supported free tier with Desktop/CLI/Web/Cloud/Chat variants.
- **Disposition**: `adopt` for the framework + SDK (pattern-donor only, no code reuse). Freebuff **Cloud product** excluded by §16.1 cloud-worker rule (joins #37/#38/#47/#61/#77/#80 on the exclusion list).
- **Top adopt-landings**:
  - `outputMode: 'structured_output'` + zod `outputSchema` per agent → the canonical §16.6 per-role result schema shape (already specified in addendum; Codebuff ships this in production).
  - Per-agent `compactContext` + `windowedFileReads` + `includeMessageHistory` flags → moves §7.1's compaction threshold from system-wide to per-role (proposal #3).
  - `publisher` field + `.agents/` folder + 150-agent marketplace → concrete shape for §14.5's skills category and proposal #2 (Skills Registry & Supply Chain).
- **Notable absence**: no decision log / immutable audit trail. LastHarness's §12.1 is *ahead* of the field here, not behind.
- **Files updated**: inline dossier in `research/surveys/01-coding-agents.md`; adoption row + §F wave-6 addendum in `research/extraction/09-feature-extraction.md`; registration log appended to `../99-corrections.md`. Run journal: this intake.


## Spec extraction PR — wave-1..6 synthesis (2026-08-30)

- **Deliverable**: 10 spec changes proposed from the wave-1..6 dossier corpus, all non-overlapping additions (no edits to locked sections), all refinements/addenda.
- **DRAFT**: [`proposed-spec-extraction-2026-08-30-DRAFT.md`](../proposed-spec-extraction-2026-08-30-DRAFT.md) — single reviewable bundle.
- **Sections proposed**: §16.10 (Owner Surfaces + new §16.10.7 worker-event stream), §14.5.A (Skills Intake Lifecycle), §7.1.B (Compaction Protocol), §16.6.B (Result Schemas v2), §16.6.A (Kernel-composed handoff packet), §12.2.A (Evidence-strength column), §14.7.A (Incident-rule DSL), §17.3.A (Below-Low tier), §16.1.A (Single-artifact packaging).
- **Origin evidence**: each section's annotation block cites the R-row or proposal number from `../11-department-blueprints.md` and the dossier file. No change without recurrence or wave evidence.
- **Invariant safety**: zero changes to §16.1/§16.5/§16.9/§12.6. All locked invariants held.
- **Blocked separately** (recorded in [`../99-corrections.md`](../99-corrections.md)): user-install vs company-operated separation for BAROOD-AI paid/ad-supported tiers — needs §17.1 addendum, not in this PR.
- **Status**: ~~pending review~~ **triaged 2026-08-30.** See below.

---

## Research program closed — 2026-08-30

The wave-1..6 extraction PR was dispositioned in [`../12-spec-extraction-triage-2026-08-30.md`](../12-spec-extraction-triage-2026-08-30.md): **7 adopt · 1 reject · 0 skip**, with five amendments required before merge. Nothing merged into `docs/archive/lastharness-v1-spec.md` pending owner sign-off.

**Correction to the "Invariant safety" line above.** It states "zero changes to §16.1/§16.5/§16.9/§12.6. All locked invariants held." That is not accurate for one section. §17.3.A (Below-Low tier) describes devices that cannot run the §16.1 local inference stack and routes their work to a remote provider, without addressing the kernel's own reasoning loop — which §16.1 makes a standing property: *"not a default that yields to a global setting — it is a standing property of the kernel specifically."* Recorded as a rejection rather than silently amended, so the reasoning stays auditable. The remaining nine sections do hold the locked invariants.

**Status of the corpus.** Research is complete. The registry, the 127 dossiers, the harvests, and the run journals now live in `archive/` — retained as the evidence trail, no longer the working set. What stays current at the top of `Research/`:

| File | Why it is still live |
| --- | --- |
| `proposed-spec-extraction-2026-08-30-DRAFT.md` | pending merge into the spec |
| `12-spec-extraction-triage-2026-08-30.md` | pending owner sign-off |
| `09-feature-extraction.md` | the adopt/note/conflict/skip catalog; consulted during implementation |
| `11-department-blueprints.md` | recurring-gap rows R1–R16; each proposed section cites one |
| `99-corrections.md` | correction ledger — append-only by nature, never closes |

**Next phase.** V1 implementation against `docs/archive/lastharness-v1-spec.md`, stack locked to Python + SQLite + Claude Agent SDK (§16.1). This file is retained as the research changelog and is no longer updated.

