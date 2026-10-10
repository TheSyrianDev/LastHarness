# 12 · Spec Extraction Triage — wave-1..6 synthesis PR (2026-08-30)

> **What this is.** A disposition pass over [`proposed-spec-extraction-2026-08-30-DRAFT.md`](proposed-spec-extraction-2026-08-30-DRAFT.md), which proposes 10 spec changes into `docs/archive/lastharness-v1-spec.md`. Method matches [`09-feature-extraction.md`](09-feature-extraction.md): every proposal is checked against the *locked* text, not against whether it sounds good. Question asked per section: *does this fit the invariants it claims to fit, and does it land somewhere V1 can actually build?*
>
> **Dispositions:** **Adopt clean** (2) · **Adopt amended** (6) · **Adopt noted** (1) · **Reject** (1) — 10 total. One claim in the draft's own summary is factually wrong and is corrected in §C.
>
> **MERGED — 2026-08-30, commit `b1c7300`.** Owner sign-off given; all 9 adopted sections merged into `docs/archive/lastharness-v1-spec.md` with the 6 amendments applied as decided. §17.3.A was rejected and is *not* merged. This file is the review record and changes no spec text itself — see the appendix for the consolidated amendment list and [`99-corrections.md`](99-corrections.md) for the merge entry.
>
> **Owner decisions recorded at merge:** (1) §14.7.A incident-rule evaluation moves to Security's `threat_detector`, not a new §8.2 exception for `monitor`; (2) owner surfaces split — console render + event stream + Class A gates on the scale-up track, Class B gates and plan artifacts deferred further; (3) Tier 1 backup in, Tier 2 a §16.4 fast-follow.
>
> **Note on §16.3:** the merge also restructured the V1 scope list into a build order (M0 spine → M1 kernel → M2+ scale-up), reversing the previous "all six departments ship at V1" decision. That reversal is recorded explicitly in §16.3, not applied silently. It changes the *item numbering* referenced in the appendix below — see the merge commit for renumbered cross-references.

---

## A. Adopt as-is

### A1. §14.5.A Skills Intake Lifecycle — ADOPT

Strongest evidence in the PR: R5×8 recurrence across waves 1–5, named as a gap twice before extraction. Closes it coherently — provenance → scan → pin → register → re-validate.

Every element is schema-legal against text already in the document: `referenced, not copied` and `versioned; never GC'd` are §6.3's own words for the Procedural type; the `skills` category exists per §14.5's 2026-08-11 addendum; §16.8 Step 2 already provisions skills. Nothing here touches §16.1, §16.5, §16.6, or §12.6.

Applying the §16.6 self-modification invariant to skills themselves is the right instinct — an imported skill is the one artifact entering the system that LastHarness did not author.

### A2. §12.2.A Evidence strength column — ADOPT

A column, correctly described as orthogonal to §12.6 Layer 1 (which grades *safety to record*; this grades *quality of evidence*). Trivial to add to the §16.1 schema, and it is what makes §16.6.B.2's append-policy auditable rather than merely asserted.

---

## B. Adopt with amendments

Each amendment below is required before merge. None is a rewrite.

### B1. §16.10 Owner Surfaces — ADOPT, amended

**Amendment: declare V1 placement, or the section is unbuildable.**

§16.3 is the V1 scope list — eight numbered items — and it does not mention §16.10. A read-only console, a three-tier approval-gate system, owner-editable plan artifacts, and a new `dispatch_event` table are four substantial additions that §16.3 does not account for. The draft says it "creates no new authority," which is true; it does not say anything about *scope*, which is the thing an implementer needs.

Fix: add item 9 to §16.3 naming which of §16.10.1–§16.10.7 are V1. Suggested split — console read-only render (§16.10.1), Class A gates (already §13.3, no new work), and the event stream (§16.10.7) are V1; Class B gates (§16.10.2) and plan artifacts (§16.10.3) are V1 candidates but must be named explicitly if so, since each carries real dispatch-path work.

**Note, not a blocker:** Class B gates park and refuse on expiry (deny-wins), while §16.6's escalation path is explicitly "scratch-scope, not pause." These are different paths — one gates *actions*, the other resolves *memory-gate ambiguities* — but the contrast should be stated in §16.10.2 so an implementer does not generalise deny-wins into the escalation path, or scratch-scope into the gate path.

### B2. §16.10.7 Worker-event stream — ADOPT, amended

**Amendment: resolve the audit-substrate contradiction the section contains.**

The section claims R10 — "append-only event streams as audit substrate" — and then specifies that `dispatch_event` is GC'd alongside `memory_working`, i.e. deleted after the 24h default retention window. An audit substrate that deletes itself is not an audit substrate. The draft softens this by noting that decision-log entries *summarising* those dispatches survive; that is true and is the right instinct, but the section should say plainly that the event stream is a **liveness** substrate and not an audit one, and leave R10's audit claim to §12.

This matters beyond wording, because **§14.7.A compiles incident rules to SQL over this exact table.** As drafted, incident detection has a 24-hour detection window: an ACL-violation attempt or a stale-skill reference is only detectable while its event survives. If that window is intended, §14.7.A must say so. If it is not, security-relevant event classes need retention longer than `memory_working`'s.

### B3. §7.1.B Compaction Protocol — ADOPT, amended

**Amendment: keep the compaction manifest out of the artifact diff.**

The protocol is sound and closes a real gap — §7.1's `context_compaction_threshold` names *when* to compact and says nothing about *what* compaction produces. "At most one compaction event per dispatch" and "the budget is not reset" are exactly the right disciplines; the latter closes a loophole where a compaction loop could quietly defeat §4.2's ceilings.

The defect is placement. The manifest is written to `.lastHarness/compaction/{dispatch_id}.yaml` **inside the worktree** — correctly, since §16.6 permits no durable state outside the worktree and MCP-mediated memory. But §16.6 Output 2 makes the worktree's `git diff` the canonical record of what a task changed, on the principle that the manager reads git rather than trusting self-report. A dispatch-internal manifest sitting in that diff pollutes the one artifact the contract treats as ground truth.

Fix: state that the artifact diff is computed excluding `.lastHarness/` (worktree-level `.gitignore`, or an explicit exclusion in the diff step). One sentence.

### B4. §16.6.B Result Schemas v2 — ADOPT, noted

Extending per-role schemas to failure channels is a genuine hardening: `refused`, `timeout`, and `failed` currently have no declared shape, so a manager parsing them is parsing prose — precisely what §16.6's 2026-08-12 addendum set out to eliminate on the success path.

The evidence-ledger append-policy ("blank beats confidently-wrong") is a good rule and consistent with §12.1.

**Note to resolve at merge:** §16.6 states the entry is "written by the dispatching manager once the worker session concludes, never by the worker itself." §16.6.B.2 says weak evidence is "pending owner or manager settlement before becoming a decision record." The owner-settlement branch needs one clarifying clause: grading affects the entry's *content and `evidence_strength`*, not its *author* — the manager remains the writer under §16.6 in all three tiers. Without that clause the two sections can be read as disagreeing about who writes history.

### B5. §16.6.A Kernel-composed handoff packet — ADOPT, amended

Making the parent→child interface explicit is right: §16.5 permits task-bounded child subagents but never says what passes between them, and the typed-packet shape matches the §E.9 meta-pattern ("the artifact *is* the contract"). Kernel-composed, never peer-to-peer, correctly extends §4.4 into the dispatch tree. The `budget` block is also where §4.6's aggregate-limits rule gets enforced in practice.

**Amendment 1 — state who spawns.** §16.5 says a worker spawns its own scoped children natively; §16.6.A says the parent submits handoff intent and the kernel composes the packet. As written it is ambiguous whether the kernel now also *spawns*. If it does, that is a change to §16.5's dispatch mechanics and needs the explicit-correction treatment §16.8's OPEN ITEM reserves for exactly this class of change. Recommended reading, and the one that keeps §16.5 intact: the kernel composes the packet; spawning stays worker-native within the existing §16.5 bounds. Say it in one clause.

**Amendment 2 — bound the read-lease.** `leases.parent_worktree_path` grants a child read access to its parent's worktree. That is fine *within one dispatch tree* and is a sensible alternative to copying state. But §16.6 Input 2 gives each dispatch a fresh worktree specifically "giving working-directory separation between concurrent tasks." The lease must be stated as intra-dispatch-tree only, never a mechanism by which concurrent dispatches come to share a worktree.

### B6. §14.7.A Incident-rule DSL — ADOPT, amended

**Amendment: the rule evaluator as drafted cannot see the data.**

This is an ACL defect, not a judgement call. The rules compile to SQL over `dispatch_event` **and `incident_log`**, and the draft assigns evaluation to the Automation department's `monitor` role (§14.6).

But §14.6 gives `monitor` a memory scope of `run_history` (write) — it has no route to `incident_log`, which is Security department memory under §14.7. §8.2 allows cross-department visibility only as "narrow, category-scoped, kernel-granted standing exceptions declared in the requesting department's own config." Automation's config currently declares none.

Two valid fixes; the first is cheaper:

1. **Assign evaluation to Security.** §14.7's `threat_detector` already holds the cross-dept standing read exception (audit logs, decision-log entries) and already writes `incident_log`. Evaluating incident rules is squarely its job, and §14.6's `monitor` keeps doing what it does today.
2. Declare a new §8.2 standing read exception for `monitor` covering exactly `dispatch_event` plus security-relevant `incident_log` fields — and add it to Automation's `department.config.yaml`.

Either way the event table's retention window (B2 above) bounds detection. State it.

Beyond the ACL: `evidence_required` is the right discipline — a rule that fires without its evidence set is a rule-engine failure — and the DSL staying deliberately narrow (match → aggregate → typed risk, with complex reasoning left to `audit_reviewer`) is correctly scoped.

### B7. §16.1.A Single-artifact packaging — ADOPT, amended

The contract is right in shape and correctly scoped: it specifies *delivery* and explicitly disclaims any change to the §16.1 boundary's post-install shape. One-file-per-platform with end-to-end checksumming is the correct extension of §5.4's department-package discipline to the installer, and the two-tier backup split — auto for the small durable state, manual for the tens-of-GB weights — is the correct rejection of auto-snapshotting model files. No invariant touched, no new authority granted, no ACL surface.

Three amendments, one note.

**Amendment 1 — the model-pack "or" is already resolved by §16.3; say so.**

The bundle carries "The selected hardware-tier model pack (§17.3) — *or* a deferred-download path for manual tier selection post-install." §16.3 item 6 settles this without the section noticing: the V1 installer does "hardware detection, **model-pack download**, `llama-swap` config generation." Download is the V1 path.

The "or" should be resolved as: **deferred download is the default; the bundled model pack is what makes the air-gapped `.iso` an air-gapped artifact.** A machine that cannot reach the network cannot defer a download — an `.iso` that deferred its model pack would not be air-gapped, which is the one property the artifact exists to have. Stating it this way also holds the build matrix at one artifact per platform rather than platforms × hardware tiers.

**Amendment 2 — two-tier backup is new scope that §16.3 does not list.**

§16.3 items 6 and 7 cover the installer and the model packs. Nothing covers backup. Tier 1 (auto: SQLite DB + key Vault entries) and Tier 2 (manual: GGUFs, skills registry, kernel config) are two new subsystems, and Tier 1 sits on the write path. Either name it in §16.3 or declare it a §16.4 fast-follow. Recommend naming it — Tier 1 is small, and it is the component that makes the claim "the backup that survives reinstalls of the app binary" true.

**Amendment 3 — the no-silent-partial-install guarantee does not reach the deferred payload.**

The section applies §5.4's checksum discipline "end-to-end" and promises "the user never sees a silent partial install." Under Amendment 1 the model pack is deferred — which means the largest single payload in the install (tens of GB, per the section's own Tier-2 rationale) arrives *after* that promise has been discharged. The deferred download needs its own checksum verification on arrival and its own retry path, or the guarantee covers the small half of the install and not the half most likely to fail mid-transfer.

**Note — Tier 1's "on every successful write" needs a cadence and a consistent snapshot.**

Tier 1 copies the database plus key Vault entries "on every successful write." Under §16.6 a dispatch may produce many writes; a full copy per write is write amplification on the hot path. Worse, the DB is WAL-backed (§16.1 stack, §12 `decision_log`), so a naive file copy of a live database can capture a torn state. Debounce or interval it, and take the copy via SQLite's backup API or `VACUUM INTO` so the artifact is internally consistent. This matters more than it looks: an inconsistent Tier-1 backup is worse than none, because it fails silently and is only discovered at restore.

---

## C. Reject

### C1. §17.3.A Below-Low tier — REJECT as drafted

**It violates §16.1, and the draft's summary claim that it does not is wrong.**

The draft's closing line states: *"None of them alter any locked invariant (§16.1, §16.5, §16.9, §12.6)."* §17.3.A alters §16.1 directly.

§16.1, verbatim:

> **Kernel locality is a standing property.** The kernel's own reasoning loop is always local, regardless of the inference provider selected during onboarding or afterward. The kernel handles sensitive onboarding intake (DOB, religion, family/emergency contacts, etc.), so locality is not a default that yields to a global setting — it is a standing property of the kernel specifically.

§17.3.A, verbatim: devices below Low *"cannot run the §16.1 local inference stack at all"* — and then routes work through "the owner's chosen inference provider (§17.2 Path A/B)."

The section addresses department dispatch routing and says nothing about the kernel's own reasoning loop. Under §16.1 that loop must be local; on a below-Low device it cannot be. The two cannot both hold, and §16.1 already anticipated the move — it says locality "is not a default that yields to a global setting." Relocating inference because the device is small is exactly a global setting.

This is not a technicality. The reason kernel locality is standing is that the kernel handles the §17.2 intake — DOB, religion, family and emergency contacts. A below-Low profile sends that reasoning to a third party. That is the precise harm §16.1 exists to prevent, on the devices least able to consent meaningfully.

**Three ways forward, in the order I'd rank them:**

1. **Park below-Low out of V1.** The §17.1 audience is "a non-technical first-time user" on consumer hardware; MCU-class and sub-4 GB devices are not that audience, and Appendix A Step 4's "Minimal fallback covers < 4 GB devices" already handles the edge without a new tier. Lowest risk, no invariant touched.
2. **Admit the exception explicitly.** If below-Low is genuinely wanted, §16.1 needs a named, owner-warned exception stating that where no local runtime exists the kernel's reasoning loop follows the provider choice — with the Step-1D-scale warning fired at install, not buried. This is a real spec correction and should be reviewed as one, not slipped in as a §17.3 addendum.
3. **Declare below-Low unsupported.** The honest option, and cheaper than either of the above. "Your device cannot run LastHarness" is a worse product sentence than the draft wants, but it is a truer one than silently relocating the kernel.

The probe itself is the genuinely valuable part — a capability probe with a generated compat matrix is good Step-0 machinery and worth keeping regardless. It is the *below-Low routing conclusion* that does not survive contact with §16.1.

---

## D. Systemic findings — gaps this PR exposes but does not close

### D1. "Verification suite" is still an undefined noun, and this PR leans on it four more times

§11.3 says "run full verification suite"; the document never says what is in it. This PR adds four more sandbox assertions (§16.10.5, §14.5.A, §7.1.B, §17.3.A) that all assume it exists. Finding §E.4 in `09-feature-extraction.md` named this; nothing has closed it.

Every Tier 1 gate in §16.8 and every assertion in this PR is a promise about a suite nobody has specified. Define it once — what runs, what counts as a pass, who may extend it — and four sections stop being aspirational. Recommend this as the next extraction, ahead of any further feature harvesting.

### D2. User-install vs company-operated is still unspecced, and it is blocking for BAROOD-AI

Carried from `09-feature-extraction.md` §F, recorded there as "the most material spec gap in the wave-6 dossier." This PR explicitly excludes it as out of scope. That exclusion is correct for this PR and wrong to leave open.

The spec defines **user-install** semantics thoroughly — self-hosted (§2.1), one Linux boundary on the owner's hardware (§16.1), cloud workers excluded (§16.1). It is silent on whether BAROOD-AI / LASTHARNESS may operate *hosted* instances as a paid or ad-supported tier. As written, that business is unspecified by the architecture it would run on: does the company see decision logs, does the user own them, does §16.1's boundary mean anything when the hardware is the company's?

Not blocking the user-install product. Blocking the revenue model. Separate §17.1 addendum; flag as the highest-value open spec question after D1.

---

## Appendix — disposition summary

| # | Section | Lands at | Disposition | Amendment required |
|---|---|---|---|---|
| 1 | §16.10 Owner Surfaces | new §16.10 | Adopt, amended | Declare V1 placement; add §16.3 item 9 |
| 2 | §16.10.7 Worker-event stream | §16.10 sub | Adopt, amended | Drop the audit-substrate claim; state the detection window §14.7.A inherits |
| 3 | §14.5.A Skills Intake Lifecycle | §14.5 addendum | **Adopt** | — |
| 4 | §7.1.B Compaction Protocol | §7.1 addendum | Adopt, amended | Exclude `.lastHarness/` from the artifact diff |
| 5 | §16.6.B Result Schemas v2 | §16.6 addendum | Adopt, noted | Manager remains the §16.6 writer in all evidence tiers |
| 6 | §16.6.A Handoff packet | §16.6 sub | Adopt, amended | State who spawns; bound the read-lease to one dispatch tree |
| 7 | §12.2.A Evidence strength column | §12.2 para | **Adopt** | — |
| 8 | §14.7.A Incident-rule DSL | §14.7 addendum | Adopt, amended | Fix the ACL: `monitor` cannot read `incident_log` |
| 9 | §16.1.A Single-artifact packaging | §16.1 sub | Adopt, amended | Resolve the deferred-download "or"; place two-tier backup in §16.3; extend the checksum guarantee to the deferred payload |
| 10 | §17.3.A Below-Low tier | §17.3 sub | **Reject** | Conflicts §16.1 kernel locality — park, or correct §16.1 explicitly |

**Counted:** 9 adopted (6 with amendments, 1 with a note, 2 clean) · 1 rejected · 0 skipped.

**Amendments required before merge, consolidated:**

1. §16.10 — declare V1 placement; add §16.3 item 9 (§16.3 already has 8 items and does not mention §16.10).
2. §16.10.7 — drop the audit-substrate claim; state the detection window §14.7.A inherits.
3. §14.7.A — fix the ACL: `monitor` cannot read `incident_log`.
4. §16.6.A — state who spawns; bound the read-lease to one dispatch tree.
5. §7.1.B — exclude `.lastHarness/` from the artifact diff.
6. §16.1.A — resolve the deferred-download "or"; place two-tier backup in §16.3; extend the checksum guarantee to the deferred payload.

**Notes to apply at merge (not blockers):** §16.6.B — manager remains the §16.6 writer in all evidence tiers; §16.10.2 — state the deny-wins vs scratch-scope contrast; §16.1.A — Tier 1 backup cadence and snapshot consistency.

**Also recorded, not extracted:** D1 verification-suite definition (recommended next extraction) · D2 user-install vs company-operated separation (needs its own §17.1 addendum).

---

*End of triage. Merge into `docs/archive/lastharness-v1-spec.md` after sign-off — the DRAFT stays unmerged and unedited until then.*
