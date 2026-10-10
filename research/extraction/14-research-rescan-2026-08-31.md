# 14 · Full Research Rescan — accuracy + missed-material audit (2026-08-31)

> **What this is.** A complete re-examination of the research corpus — **205 archived files**, **7 live files** (6 pre-existing + this rescan), **147 tool dossiers**, one **146-row** feature ledger — run *after* the wave-1..6 merge (`b1c7300`) and the full-spec review (`7fc0120`). Two questions: **is the research accurate?** and **did we miss anything worth adding?**
>
> **Method.** Mechanical first (parse every ledger row, count every dossier, match every entry), then evidence-based absence testing: for each candidate mechanism, grep `docs/archive/lastharness-v1-spec.md` for its distinctive terms. A mechanism is reported as *missed* only when the term count is **0** — not "I don't remember seeing it." Three parallel agents read all 142 dossiers independently; every claim below was re-verified by grep before being recorded.
>
> **Scope note.** This rescan covers the *research corpus*, not the spec. Spec-side findings stay in [`13-spec-review-2026-08-30.md`](13-spec-review-2026-08-30.md).

---

## A. Accuracy — what held up

The substantive claims are sound. These were verified against primary text, not taken on trust:

| Claim | Where made | Verified |
|---|---|---|
| §16.1 kernel-locality passage quoted verbatim | triage §C1 | ✅ byte-identical in spec (line 1059) |
| DRAFT closes with *"None of them alter any locked invariant (§16.1, §16.5, §16.9, §12.6)"* | triage §C1 | ✅ verbatim at DRAFT line 381 |
| §14.6 `monitor` has memory scope `run_history` (write), no route to `incident_log` | triage §B6 | ✅ stated in merged §14.7.A (line 1009) |
| Amendment B3 — artifact diff excludes `.lastHarness/` | triage §B3 | ✅ in §16.6 Output 2 |
| Amendment B6 — evaluation moved to `threat_detector` | triage §B6 | ✅ applied, with the ACL rationale |
| Six locked invariants unmodified through the merge | review §A | ✅ (prior pass, `HEAD~2` byte-diff) |

**The triage is accurate.** Its reasoning, its quotes, and its amendments all survive contact with the primary sources. No disposition was overturned.

---

## B. Accuracy — six count defects

Every one of these is *stale*, not fabricated. The pattern is uniform: **counts are written when a wave lands and never updated when later waves append.** The corpus grew under its own bookkeeping.

| # | Defect | Stated | Actual |
|---|---|---|---|
| B1 | `09-feature-extraction.md` header disposition counts | adopt 29 / note 40 / conflict 12 / skip 27 = **108** | **39 / 58 / 17 / 32 = 146** |
| B2 | `archive/00-index.md` line 19 disposition counts | adopt 33 / note 53 / conflict 13 / skip 30 = **129** | **146** — the 129 figure was correct **at entry #130** and went stale when wave 5 landed |
| B3 | `09` appendix heading | *"all 108 active entries"* | heading sits over a **146-row** ledger |
| B4 | wave-3 dossier count (`00-index` **and** `11-department-blueprints` §Wave 3) | **51** | **56** — correct at 51 before the drain batch (#132–136) appended |
| B5 | `09` ledger footer | *"#45/#38 merged"* | #45 correctly absent, but **#38 is still a live `note` row** (line 157). #115/#129 delisted yet retained as `skip` rows. Arguably deliberate audit-trail retention, but the footer's framing contradicts the rows' presence. |
| B6 | `11-department-blueprints` §Wave 3 | *"Registry coverage is now COMPLETE — all 130 active entries have wave-level dossiers"* | **3 entries have no dedicated dossier**: #116 Capptivo, #118 Roboflow RF-DETR, #127 holaOS. #127 is covered indirectly via `wave4/qwenpaw.md`. |

**Why B1/B2 matter more than they look.** These are the two numbers an implementer reads to gauge how much of the field was covered. Both understate the corpus by ~13%, and they disagree with each other by 17 entries — so a reader comparing them gets a third, nonexistent answer.

**Corpus inventory (verified):** wave1 40 · wave2 36 · wave3 56 · wave4 10 = **142 dossiers**, plus 5 in `raw-sources/` = 147 `.md` files. Ledger: **146 rows**, entries #1–147 with only #45 absent (merged into #44).

---

## C. Missed material

Ranked by how much they'd change what gets built. Every "0 hits" below was verified by grep against `docs/archive/lastharness-v1-spec.md`.

### Tier 1 — these shape M0

**M1. Lifecycle hooks as the enforcement substrate.** *The largest miss in the corpus.*

Sources: adopts **#2 ECC, #29 Deepseek Harness, #44 oh-my-openagent** — three tools converging independently — and it is meta-pattern **§E.1** in `09`: *"Hooks are the enforcement substrate… one sentence in §16.6 prevents ad-hoc gate scattering."*

**`hook` appears 0 times in the entire 2,137-line spec.** The spec specifies plenty of gates — deny-by-default tool lists (§16.6 Input 4), a deterministic memory gate (§16.6), ACL layers (§8.2), a sanitization gate (§12.6) — and never names the substrate that enforces them. M0 is about to implement all of them, ad-hoc, with no shared mechanism.

**M2. C2's fix is already sitting in the corpus, unharvested.** *The most useful single finding.*

Open spec finding **C2** says §16.6.B.1's success schema needs `commit_sha` but nothing produces it — §16.6 Output 2 makes the worktree `git diff` canonical, and §14.2's `merge_coordinator` runs `merge --no-ff --no-**commit**`.

Adopt **#91 Aider** is *"git commit-per-changeset inside worktree runs"* (`commit-per-changeset`: **0 hits**). Committing per accepted changeset **inside the worktree** resolves C2 cleanly: the commit is local to the dispatch, `commit_sha` gets a producer, the diff stays canonical, nothing pushes, and §14.2's no-commit rule is untouched because that governs the *merge*, not the dispatch. It also delivers free rollback checkpoints — which is R10's evidence note (*"aider auto-commit-per-edit trail"*) and pairs with #93 Cline's shadow-git checkpoints (**0 hits**).

**M3. §16.6 Output 3 has no schema.** Adopt **#64 LangFuse** — span-per-call trace schema (tool invocations, token counts, latencies), to be *"defined once in §16.6 Output 3."* Output 3 is currently one line: *"Transcript/trace — feeds the Rationale and Evidence fields."* `trace schema` and `span-per-call`: **0 hits**.

M0 builds the dispatch path; the trace is one of its four declared outputs. The spec itself already notices the gap at line 1516 — §16.10.7 needs *"a finer-grained substrate than terminal status"* — but that is the *liveness* stream, not the trace schema.

**M4. Degenerate-loop guards with force-termination.** Adopt **#59 T-search** — `free_text_guard_limit`, `locked_search_streak_limit`, `max_turns_per_round`, degenerate-loop force-terminators. `runaway` · `infinite` · `watchdog` · `guardrail`: **all 0 hits**. M0 dispatches unattended ephemeral workers with no breakers at all.

### Tier 2 — high value, lands at M1/M2

**M5. Untrusted-content fencing + injection scan at ingress.** Adopt **#4 TokenGoat**; independently re-derived in wave 3 by **gstack** and **yc-qm** (provenance-labelled admission gate, datamarking, hidden-element/ARIA stripping, URL blocklist, labelled untrusted envelope). `injection` · `fence` · `envelope` · `admission`: **all 0 hits**.

Research workers will ingest untrusted web content with **no read-side guard whatsoever**. §12.6 is a *write-side* gate; there is no counterpart on the way in.

**M6. Decomposition containment — depth/node/branching caps + lineage cycle detection.** R8, source **#107 pi-rlm** (`maxDepth`/`maxNodes`/`maxBranching` + lineage cycle detection). `recursi` · `cycle detect` · `maxDepth`: **all 0 hits**.

§16.6.A gives each child a `budget` block, but there is no depth cap and no cycle detection anywhere. §4.6's aggregate-limit rule is the only containment in the document.

**M7. Fail-closed pre-compute gate with structured `BLOCKED`.** wave-3 **img2threejs** — `--strict-quality` validates before generation; on failure returns `BLOCKED` with metrics, causes, next-action, and writes no artifact. The spec's single `fail-closed` (line 984) is an analysis posture, not a gate. Refusing work *before* spend is the pattern §11.3 currently lacks.

**M8. Scripts-enforce / model-judges split, and an Independent Auditor role.** wave-3 **img2threejs** (~90 stdlib-only modules own validation, gating, evidence packaging; models only judge and write code) and **LongHorizon** (Auditor inspects real files/tests with its own tool scope and timeout; final reply written from verified state alone). Together these are a concrete, non-LLM division of labour for the undefined verification suite — they feed **C1** directly.

**M9. Egress: fail-closed pre-send receipt, and a 4-valued content policy.** wave-3 **gstack** — hash-chained tamper-evident receipt written *before* every off-machine send; sinks refuse to send if the write fails. `receipt` · `tamper`: **0 hits**; §7.1 logs egress after the fact but never gates it. wave-4 **ruflo** adds a 4-valued policy (BLOCK/REDACT/HASH/PASS) where the gateway is currently binary allow/deny.

⚠️ **Conflict to resolve before adopting:** the spec has one `REDACT` hit and it currently *rejects* redaction. Adopting the 4-valued policy means revisiting that line — it is a real decision, not a wording fix.

### Tier 3 — genuine, later

Grouped by landing slot. All verified absent.

| Slot | Mechanism | Source |
|---|---|---|
| §12.4 | Typed causal edges (`CAUSED`/`INFLUENCED`/`PRECEDENT_FOR`) + precedent query | semantica (w2) |
| §4.4 | Crash-safe work-queue leasing — `FOR UPDATE SKIP LOCKED`, lease expiry, `dueAt` | comp-crm (w2) |
| §12.2 | Per-role cryptographic signing identity on log rows | buzz conflict-sketch (w2) |
| §16.6 | Grammar-constrained decoding for tool calls | needle-2 (w1) |
| §13.2 | Checkpoint / replay / fork (§13.2 is currently five lines) | langgraph (w1) |
| §16.6 Input 2 | Dispatch-scoped filesystem projection — mount allowed slice at spawn, tear down at teardown | cloudflare-computer conflict-sketch (w2) |
| §14.2 | Bounded post-edit lint/test verify-and-repair | aider (w1) |
| §4.2 | Spend-after-validated-write-back — skips/preflight/dry-runs cost nothing | loopx (w4) |
| §16.6 | Tool-result compressors + size-regression guard (never larger than original) | 9router (w4) |
| §14.6 | Enumerated when-to-interrupt-human triggers + dedup-before-escalate | oh-my-hermes (w2) |
| §14.3 | `sources_reviewed` as a required research-completion field | exa (w2) |
| §16.5 | Worker trust scoring from logged outcomes → dispatch breadth | ruflo (w4) |
| §14.7.A | Rule analysis operators (threshold / outlier / first_collection_only / match_all_to_first) | spiderfoot (w2) |
| §16.10.7 | Frozen run snapshots + evidence playback | hermes-studio (w4) |
| §16.10.2 | Trust-gated resume bindings (signed prefix, dir+env fingerprint, secrets stripped) | cmux (w4) |
| §16.6 | Batch-run descriptor + completion-grace for hung children | sandcastle conflict-sketch (w2) |
| §12.2.A | Evidence FRESH/STALE/MISSING + tree fingerprint (extends strength grading with freshness) | gstack (w3) |
| §8.2 | Hard-denial floor no posture relaxes (recursive deletes, destructive SQL) | yc-qm (w3) |
| §14.5.A | Invocation-class direction constraint (user-invoked skills may call model-invoked, never another user-invoked) | matt-pocock (w3) |
| §6.1 | Anchor predicates re-validated at recall + trust decay and pinning | tokengoat-mem (w1) |

### Two stale proposals found along the way

**The "dispatch-flow family" note cannot land where it says.** `09` §B proposes recording *"decompose → gate → fan-out"* — a four-source convergence (GStack, Pocock, Cline, Deep Research) — as *"a §16.9 dispatch-flow family."* **§16.9 is already "Department Manager."** The slot is occupied; the note has no home. `dispatch-flow`: 0 hits.

**The triage's own instruction was never executed.** Triage §C1 rejected §17.3.A but said explicitly: *"The probe itself is the genuinely valuable part — a capability probe with a generated compat matrix is good Step-0 machinery and worth keeping regardless."* `capability probe` and `compat matrix`: **0 hits**. §17.3 has a graceful degradation rule (line 1668, *"No GPU detected → treat as CPU-only"*) but no probe primitive. The rejection landed; the salvage the triage asked for did not.

---

## D. What this means

**The research is accurate.** No disposition overturned, no quote falsified, no invariant violated by the merge. The six count defects in §B are bookkeeping rot, not bad analysis — and each has a single verifiable correct value.

**The research is under-harvested.** The DRAFT carried 10 proposals for a 147-tool corpus. The corpus contains substantially more, and the gap is not random — it follows a pattern:

- **Convergent mechanisms are better evidenced than what shipped.** M1 (hooks) has three independent sources plus a named meta-pattern, and is *completely* absent. M5 (ingress fencing) was independently re-derived by three tools across two waves.
- **Conflict-tier salvage was systematically dropped.** A `conflict` verdict buries the compliant re-implementation sketch alongside the violation. §C's Tier-3 table contains four such sketches (cloudflare-computer, sandcastle, buzz, and indirectly yc-qm) — all salvageable, none harvested.
- **Open spec findings have answers already in the corpus.** M2 resolves C2 outright. M7 and M8 feed C1. That is worth more than finding new features.

**Against an MVP build order**, the ranking changes. Tier 1 (M1–M4) is what M0's implementation needs — hooks are *how* the gates get built, the trace is one of four declared dispatch outputs, loop guards are dispatch safety, and M2 closes a known open finding. Tier 2 and 3 are scale-up track and can wait without cost.

## E. Recommended next step

**Harvest Tier 1 into the spec before M0 code** — four items, and M2 closes an existing open finding rather than adding scope. M1 is one sentence in §16.6; M2 is one clause plus a §14.2 clarification; M3 is a schema block; M4 is three named limits.

Separately, **correct the six counts in §B** — mechanical, and each has one right answer.

Both are decisions for the owner, not actions taken unilaterally: `09-feature-extraction.md` and `archive/00-index.md` are historical records, and the count corrections should be logged in `99-corrections.md` rather than silently edited.

---

*End of rescan. No spec text was modified by this pass.*
