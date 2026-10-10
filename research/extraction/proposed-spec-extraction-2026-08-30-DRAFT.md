# Proposed Spec Extraction — Wave-1..6 Synthesis (DRAFT, 2026-08-30)

> **Purpose.** Consolidated, reviewable draft of all spec-side changes implied by the wave-1..6 dossier corpus, ready for one merge pass into `docs/archive/lastharness-v1-spec.md`. Each section is annotated with the originating research evidence so a reviewer can verify the change is supported.
>
> **Convention.** Each numbered subsection below is a *proposed patch* to `docs/archive/lastharness-v1-spec.md` — the heading shows where it lands in the document. The DRAFT is **not merged**; review and merge separately.
>
> **Provenance.**
> - §16.10 first drafted 2026-08-24 (`proposed-section-16-10-DRAFT.md`), here lightly sharpened with wave-5/6 evidence.
> - §7.1 Compaction Protocol addendum: wave-1 R3×5 (token-goat, context-mode, ecc, deerflow, scrapling), wave-3 strengthenings.
> - §14.5 Skills Intake Lifecycle addendum: wave-1 R5×8 (skill-pack supply chain), wave-2 oh-my-hermes + qiaomu-seo freshness, wave-3 pin/quarantine strengthenings.
> - §16.6 Result Schemas v2 addendum: wave-1 R6×3 + R10×4, wave-2 comp-crm ledger pricing, wave-3 Impeccable detector pattern.
> - §17.3 Below-Low tier extension: wave-2 lemonade machine-capability probe (R15), wave-2 needle-2.
> - §12 Decision-log append-policy: wave-2 R12 (comp-crm write/suggest/prefer-blank thresholds).
> - §16.6 Kernel-composed handoff packet: wave-2 R11 (ai-memory + munder-difflin + LoopX #138 strengthen).
> - §14.7 Incident-rule DSL: wave-2 R13 (spiderfoot YAML correlation rules).
> - §17.3 Hardware-capability probe: wave-2 R15 (lemonade, local.ai).
> - §16.1 Single-artifact packaging contract: wave-2 R16 (pocketbase one-binary shape).

---

## §16.10 — Owner Surfaces

> **Status.** Re-drafted from `proposed-section-16-10-DRAFT.md` (2026-08-24) with wave-5/6 cross-references and one explicit addition. Merge as the canonical §16.10 of `lastharness.md`.

### 16.10 Owner Surfaces

> **Added — 2026-08-30.** Proposed as blueprint #1 of the feature-exploration program (`research/extraction/11-department-blueprints.md`) and adopted on recurrence evidence: approval gates before side-effectful actions appear in 6 of 127 surveyed tools, editable plan-before-execute phases in 5, owner visibility surfaces in 4+, with remote-control auth ladders concurring independently (full dossiers under `research/surveys/10-feature-dossiers/`). This section creates a **render-and-gate surface**. It creates no new authority, no standing process, no new transport, and no exception to any invariant above: §16.5's ephemeral-dispatch rule, §4.4's kernel mediation, §16.4's messenger exclusion, and §7.2's override mechanics all hold unchanged.

The owner currently observes the organization through notifications (§14.6) and the decision history (§12) — both after the fact. §16.10 specifies the during-the-fact surface: a read-only console over kernel state, a contract for gating defined action classes behind owner confirmation, and an owner-editable plan artifact that makes two-phase dispatch inspectable. Every element is a presentation over state that already exists in the decision log, the department registries, and the dispatch contract; nothing here originates state.

#### 16.10.1 Owner Console

The console is a **read-only render** of kernel-owned records. It has no mutation endpoints; its entire authority is SELECT-shaped.

| Rendered view | Data feed (existing) |
| --- | --- |
| Routing queue depth, queued/refused tasks | Kernel routing queue (§4.4) |
| Active dispatches: worktree, task spec, elapsed vs ceilings | Dispatch records (§16.6 inputs 2, 7) |
| Per-task status: working / blocked / idle | Terminal-status stream (§16.6 Output 1), sampled |
| Decision-log queries (filter by dept, type, time, Previous ID chains) | Immutable log (§12) |
| Owner notification queue with parked/expired items | Notification queue (§14.6 `alert_dispatcher`) |
| Approval and plan artifacts awaiting action (§16.10.2–§16.10.3) | Staged artifacts (this section) |

Delivery ownership is unchanged: §14.6's `alert_dispatcher` still owns reaching the owner (immediately if reachable, queued otherwise); the console renders the same queue and adds nothing to its authority. Where the owner is unreachable, §13.2 step 5's notify-at-next-interaction pattern applies unchanged.

#### 16.10.2 Approval Gates

Action classes are gated by a three-tier classification, configured per role in the dispatch config slice like every other policy (nothing hardcoded, §2.1):

| Class | Definition | Gate |
| --- | --- | --- |
| A — Irreversible | Pushes to protected branches, releases, production deploys, purges, retirement (§13.3; §14.2 invariant; §16.8 retirement) | Existing §13.3 mechanics: biometric/key-based authorization. Unchanged by this section. |
| B — Side-effectful but reversible | Out-of-worktree writes, external API calls through the §7.1 gateway, cross-department config proposals, bulk deletes within memory | New: staged behind owner confirmation before execution |
| C — Routine in-scope work | Everything inside the role's allow-lists, worktree, and budgets | None (normal dispatch, §16.5/§16.6) |

**Gate semantics.** A Class B action is *staged*, never executed inline: the worker (or manager) returns a typed approval request containing the exact target, the diff or payload preview, the requesting role, and the §12-linked task context — mirroring §12.6's echo-back discipline, which forces engagement with the specific record rather than a memorized yes. The owner resolves with a closed response vocabulary: **approve / amend / deny / park**. Approve executes through the normal kernel path and logs a decision (§12, Initiator: owner); amend returns the artifact to the originating manager as an amended task spec; deny refuses and logs.

**Unreachable-owner behavior.** An unresolved gate parks in the notification queue (§14.6) with a configurable expiry. On expiry the staged action is **refused** — deny-wins, never approve-wins — and the refusal is logged as a decision (Outcome: expired-unapproved). Parked-but-unexpired gates survive disconnection and are resurfaced on reconnect (resumable cards below).

#### 16.10.3 Plan Artifacts

Two-phase dispatch becomes available to any manager for tasks flagged `planned` (always required for Project Agents, §4.6):

1. **Propose.** The manager emits a **plan artifact**: the decomposed task graph, target departments and roles, per-node budgets, acceptance criteria, and dependencies — stored project-scoped in Information exactly as §4.6 already specifies for Project Agent graphs.
2. **Gate.** Execution fan-out is blocked until the owner approves or amends the artifact. Amendment is first-class: the owner edits nodes, budgets, or targets; the amended plan re-enters Step-4-style validation (§16.8's Evolution Sandbox pass) when changes are material, then proceeds.
3. **Execute.** Approved plans dispatch as ordinary kernel-mediated work (§16.5/§16.6); progress renders in the console (§16.10.1).

The plan artifact is a **projection** of state Information already holds (§4.6: task graph, node status, replan history live there) — this subsection makes it owner-editable; it creates no second store.

#### 16.10.4 Transport and Authentication

All owner-surface traffic rides §13.4's connectivity rules without addition: encrypted, owner-initiated tunnels only; every remote session logged with IP, duration, actions.

**Pairing-token ladder.** A device joins by consuming a one-time pairing token generated in-session, yielding a named device session bound to the owner's identity; sessions are individually revocable, and the revocation ledger is itself append-only. Compromised-token blast radius is one device session, revocable in one action.

**Resumable approval cards.** Gates raised while the owner is disconnected are persisted as staged decisions (§12-adjacent records), not held in process memory — so they survive restarts and reconnects and cannot be lost by a crashed session.

**Explicitly excluded:** no standing daemons or listener processes (§16.5); no third-party cloud relay of console or gate traffic (§16.1); no messaging-channel listeners — Slack/Telegram surfaces remain out of V1 per §16.4, whose post-V1 adapter shape (owner-initiated tasks mapped to kernel routing) is the only sanctioned future path.

#### 16.10.5 Tier-1-Equivalent Invariant

**Render-don't-mutate:** the console renders kernel state but cannot mutate it; every mutation path — gate approvals, plan amendments, overrides — is a kernel-mediated owner decision recorded per §12, and staging surfaces can refuse or hand off but never execute directly.

*Sandbox assertion (§11.3):* clone state; open the console against the clone. (1) Attempt a decision-log edit, a department-config write, and a worker spawn directly from console context. (2) Raise a Class B gate, kill the transport mid-confirmation, reconnect, and resolve. (3) Present an expired gate and attempt late approval. PASS = all direct mutations refused at the surface; the interrupted gate resumes as staged-not-executed; the expired gate resolves only as logged refusal; every accepted action appears as a §12 record with Initiator: owner routed through the kernel. FAIL = any state change observable without a corresponding kernel-mediated decision record.

#### 16.10.6 Non-Goals

This section deliberately does **not**:

- create standing processes, resident consoles, persistent inboxes, or agent-to-agent messaging — the §16.8 OPEN ITEM constraint is restated here and binds this surface doubly;
- contact workers directly — the manager remains the single dispatch authority (§16.9), and console-originated work enters exclusively as kernel-routed tasks;
- alter §7.2 override mechanics — overrides remain logged interventions, and the console is their presentation, not their replacement;
- introduce transport outside §13.4, or messaging-channel ingress per §16.4;
- serve as a workflow engine or scheduler — recurring work belongs to Automation (§14.6), and the console renders job state without defining jobs;
- add telemetry, analytics, or phoning-home of any kind (§13.4.2 applies to every byte).

#### 16.10.7 — *New in this revision.* Worker-event stream substrate

> **Added — 2026-08-30 (wave-6).** Strengthened by Codebuff #147's `CodebuffClient.handleEvent(event)` typed event stream as production prior art — every tool call, every agent start/finish, every error emits a typed event. The §16.6 Output 3 "transcript/trace" line is the dispatch-completion render; the §16.10 console's *during-the-fact* status feed needs a finer-grained substrate than terminal status.

The console's per-task status (§16.10.1, "working / blocked / idle") is sampled from the dispatch's *terminal status* (§16.6 Output 1). For the during-the-fact view to be honest about long-running dispatches, an event stream substrate feeds the console between terminal-status reads:

| Event class | Source | Console render |
| --- | --- | --- |
| `dispatch.started` | Kernel at §16.6 inputs-set | "Routing…" → "Working…" |
| `worker.tool_call` | Worker's MCP handle (§16.6) | Tool name + elapsed, sampled |
| `worker.thinking` | Same | Idle dot, optional |
| `dispatch.escalated` | §16.6 ambiguity-escalation path | "Blocked — awaiting kernel" |
| `dispatch.terminal` | §16.6 Output 1 read | Final status, transitioned out of "working" |

The stream is **append-only**, lives in the same SQLite database as the decision log (§12.1, §16.1) but in a separate `dispatch_event` table keyed by `(dispatch_id, sequence)` — so a console render is a windowed query against ordered events, not a probe against a live process. This satisfies R10 (append-only event streams as audit substrate) without introducing a new transport or any standing process. The event table is GC'd alongside `memory_working` (§6.3) — events older than the retention window are deleted; the decision-log entries that *summarize* those dispatches (§16.6) are kept forever per §12.1.

This adds **no new authority** (§16.5/§16.6 still govern dispatch). It specifies a substrate that the console reads. The worker's existing MCP-handle writes (§16.6 memory writes) and decision-log writes (§12.6 sanitization gate) are unchanged; only their *visibility* to the console is enriched.

---

## §14.5 Skills Intake Lifecycle addendum

> **Status.** New addendum to §14.5 (Information / Memory / Knowledge / Skills). Lands as `§14.5.A Skills Intake Lifecycle` immediately after the existing `§14.5 addendum 2026-08-11 (Team Memory categories reconciled)` paragraph.

### 14.5.A Skills Intake Lifecycle

> **Added — 2026-08-30.** Evidence: R5×8 in `research/extraction/11-department-blueprints.md` §1 (skill-pack supply chain the single most-recurred cluster across waves 1–5), strengthened by wave-2 oh-my-hermes 36-skill layer + qiaomu-seo strict-stale auto-block, wave-3 pin/quarantine-before-trust (agent-skills-flow vendir lock, gstack domain-skill quarantine, elevenlabs trigger-eval-before-register), and wave-5 provenance tagging convergence (codegraph `synthesizedBy`, loopx evidence-strength, ruflo signed witnesses). Pre-extraction gap was named twice before; this addendum closes it.

The `skills` category already exists in §14.5 and is provisioned via §16.8 Step 2. What it lacks is the *intake lifecycle* — provenance → scan → pin → register → re-validate — that the recurrence evidence demands. This subsection specifies that lifecycle.

**Provenance declaration.** Every skill registered in the `skills` category carries a `publisher` field identifying who authored it, mirroring Codebuff #147's typed `AgentDefinition.publisher` (wave-6 prior art). First-party skills from BAROOD-AI / LASTHARNESS share the org publisher; community skills declare their own. Provenance is recorded in the registry, not in the skill body, so provenance can be updated without touching the artifact.

**Sandbox scan (one-time, blocking).** Before a skill enters the registry, it runs through a deterministic scan that examines the SKILL.md frontmatter, the script bodies, the references list, and the assets list. The scan enforces the §6.1 Procedural-type retention invariants (§16.8 Step 2 already names this), verifies that declared dependencies are present, and refuses skills that attempt to widen their own scope (the §16.6 invariant applied to skills themselves). A skill that fails the scan is not registered; the owner is notified per §14.6 with the failure reason.

**Version pinning.** Once registered, a skill is bound to a specific versioned artifact — a content hash plus the upstream publisher + version string when one exists (e.g. `npx skills add owner/repo --skill foo@1.2.3`). The skill is *referenced*, not copied, per §6.3; pinning happens in the registry record. A pinned skill is unchanged in the registry unless explicitly updated via the same lifecycle.

**Lazy loading.** A skill's content is not loaded into a worker's context by default. Per the `skills` category's role in §16.5's per-dispatch input gate, a skill is loaded *only when a dispatched worker explicitly invokes it* (via the role's tool allow-list or via a manager's pre-dispatch excerpt). This pairs with §7.1's `context_compaction_threshold` (see §7.1.B Compaction Protocol below) to keep the working context small.

**Freshness governance.** A registered skill has a freshness window — a TTL on the *stale-by date* of any external source it cites (URLs, package versions). qiaomu-seo's dated-source registry (wave-2, #128) is the prior art: a skill citing an external URL that exceeds its freshness window is flagged for owner review via §14.6, not silently relied on. The window is per-skill, set by the publisher at registration; skills without a declared window default to the kernel's policy default (§7.1).

**Re-validation on update.** When a pinned skill's upstream version bumps, the registry does not auto-update; it stages the new version, re-runs the sandbox scan, and presents the diff to the owner (§16.10.2 Class B gate — registry update is side-effectful but reversible). On owner approval the new version is bound; on deny the old version remains.

**Re-validation on incident.** §14.7's incident rule DSL (§14.7.A addendum below) can fire a "stale or unsafe skill" rule; when triggered, the named skill is suspended pending re-validation. Suspension is a status change in the registry, not a deletion — re-validation restores it; a hard removal follows §16.8's department retirement procedure applied to a single skill.

*Sandbox assertion (§11.3):* clone the registry; (1) register a skill whose `publisher` field is missing; (2) register a skill whose scan reveals a credential-shaped string in a script body; (3) update a pinned skill to a new version without an owner gate. PASS = (1) refused at registration, (2) refused at scan, (3) refused at update with owner gate required. FAIL = any of the above auto-proceed.

---

## §7.1 Compaction Protocol addendum

> **Status.** New addendum to §7.1 (Policy and Governance → `resource_limits`). Lands as `§7.1.B Compaction Protocol` immediately after the existing 2026-08-12 addendum paragraph (which already adds `max_output_tokens_per_task` and `context_compaction_threshold`).

### 7.1.B Compaction Protocol

> **Added — 2026-08-30.** Evidence: R3×5 in `research/extraction/11-department-blueprints.md` §1 (compaction-time structured manifests & resumable snapshots), wave-3 strengthenings, wave-5 LoopX #138 quota-gated ticks + Codebuff #147 per-agent `compactContext` + `windowedFileReads` flags as production prior art. The existing `context_compaction_threshold` (§7.1) names *when* to compact; this subsection specifies *what compaction produces* and *how dispatch continues from it*.

The existing §7.1 line "compact the worker context at this share of the ceiling, before failure" specifies a trigger; it does not specify a protocol. This subsection closes that gap.

**Trigger.** When a worker's context reaches `context_compaction_threshold` of its `max_tokens_per_task` ceiling, the dispatch contract (§16.6) fires compaction once per dispatch — at most one compaction event per dispatch lifecycle, with one exception below. The threshold is per-dispatch, not per-step; repeated low-size dispatches do not accumulate compaction events.

**Compaction artifact.** Compaction produces a **compaction manifest** — a structured summary of the worker's context above the threshold, persisted in the dispatch's worktree as `.lastHarness/compaction/{dispatch_id}.yaml`. The manifest carries:

- **Preserve list** — facts the worker has explicitly marked must-not-summarize (user-specified identifiers, in-progress file edits, the active task spec).
- **Drop list** — entries the worker asserts are no longer relevant (already-applied edits, expired tool results).
- **Summary** — a structured compression of everything else, with a token count.

Compaction is *lossy* by default; the preserve-list discipline is how lossy compression stays safe. The worker that marked items as must-not-summarize is the same worker that resumes after compaction, so the marking decision lives with the dispatch.

**Resume.** Compaction is followed by a *compaction step* — a single LLM call that summarizes the manifest into the new context, then resumes the dispatch with the compacted context as its working set. The dispatch's budget (`max_tokens_per_task`) is not reset by compaction; the worker continues under the same ceiling with a smaller context.

**Compaction exception.** A second compaction in the same dispatch is permitted only when the dispatch is a Project Agent (§4.6) and the second compaction happens at a node boundary where a fresh subagent is about to spawn. The second compaction is then part of the handoff packet (§16.6.A Kernel-composed handoff packet) rather than mid-task.

*Sandbox assertion (§11.3):* dispatch a task that exceeds `context_compaction_threshold` with a preserve-list including an in-progress file edit. (1) Verify the manifest exists with the edit entry in preserve. (2) Verify the worker resumes with the edit still in progress, not lost. (3) Verify the dispatch's `max_tokens_per_task` was not reset. PASS = compaction manifest present with correct preserve-list, dispatch resumes correctly, budget unchanged. FAIL = edit lost, or budget reset, or no manifest.

---

## §16.6 Result Schemas v2 addendum

> **Status.** New addendum to §16.6 (Worker Dispatch Contract). Lands as `§16.6.B Result Schemas v2` immediately after the existing 2026-08-12 addendum (which already specifies per-role result schemas). This addendum extends that line with typed failure channels and the evidence-ledger append-policy.

### 16.6.B Result Schemas v2

> **Added — 2026-08-30.** Evidence: R6×3 (structured partial-failure channels & typed errors) + R10×4 (append-only event streams as audit substrate) + R12 (evidence-ledger append-policy), wave-2 comp-crm write/suggest/prefer-blank thresholds, wave-3 Impeccable #132 deterministic detector pattern as production prior art. The existing 2026-08-12 addendum specifies per-role *success* return shapes; this addendum specifies per-role *failure* return shapes and the evidence-ledger policy that decides what gets written to the decision log.

#### 16.6.B.1 Typed failure channels

Per the existing 2026-08-12 addendum, every role's `department.config.yaml` carries a `result_schema` field for success returns. This addendum extends the same config slice with a `failure_schema` field, also a typed object specifying the shape of `failed` / `refused` / `timeout` returns. The schema declares which fields a manager can rely on for *each* terminal status:

```yaml
role: backend_developer
result_schema:
  success: { type: object, fields: { commit_sha: str, files_changed: [str], diff_summary: str } }
failure_schema:
  refused:
    type: object
    fields: { reason_code: enum[out_of_scope|invariant_violation|insufficient_evidence], explanation: str, evidence_refs: [decision_id] }
  timeout:
    type: object
    fields: { elapsed_s: int, partial_diff: str, recoverable: bool }
  failed:
    type: object
    fields: { exception_class: str, stack_tail: str, last_step: str }
```

A manager that receives a `failed` return without a `last_step` field is a schema violation — that role's spec is incomplete, not a worker failure. The §16.8 Tier-1 going-live gate applies to schemas as well as behaviors.

#### 16.6.B.2 Evidence-ledger append-policy

Every dispatched task eventually produces §12 records. The decision log is *append-only* and *immutable* (§12.1), but the **strength** of what is written is not uniform. Wave-2 comp-crm's evidence-ledger pricing is the prior art: writes are priced by the *confidence* of the underlying evidence, not by whether a worker *wants* to write.

This subsection specifies a three-tier append-policy:

| Confidence | Append behavior | Source |
| --- | --- | --- |
| **Strong** | Auto-write to decision log on task completion | Test results, build outputs, LSP diagnostics, validator reproductions, decision-log entries referenced by the dispatch |
| **Weak** | Surface as a *suggestion* in the owner notification queue (§14.6), pending owner or manager settlement before becoming a decision record | LLM-inferred outcomes, free-text rationale, prose self-reports, unverified worker claims |
| **Blank** | Do not write. Prefer empty evidence over confidently-wrong evidence — a "blank beats confidently-wrong" rule | Worker self-assessments without machine-verifiable backing |

The kernel-side gate that enforces this lives in the Information department's memory handle (§16.6, §14.5). The gate is deterministic, not a model call. Workers that emit only blank-evidence outputs produce minimal decision-log entries (`Decision Type: blank-completion, Outcome: completed, Evidence: none`) — the log records the *fact* of completion, not an unsupported claim about quality.

This policy pairs with §16.6.B.1: a `failure_schema.reason_code` field is itself strong evidence (the worker's own typed refusal); a `failure_schema.explanation` free-text field is weak evidence.

---

## §12 Decision-log append-policy (R12 landing)

> **Status.** Lands as a new paragraph at the end of §12.2 ("What Is Recorded"). Specifies the evidence-grading column on the `decision_log` schema.

### 12.2.A — Evidence strength on the decision log

> **Added — 2026-08-30.** Evidence: R12 (evidence-ledger append-policy), wave-2 comp-crm strong/weak/blank pricing, wave-3 Impeccable detector pattern. Pairs with §16.6.B.2 (per-task append-policy); this is the *column* that records which tier a given decision-log entry was written at.

The decision log schema (§16.1) gains an `evidence_strength` column with the three-tier enum from §16.6.B.2:

| `evidence_strength` | When written | Reader expectation |
| --- | --- | --- |
| `strong` | Auto-write on task completion with machine-verifiable backing | Treat as authoritative for §12.4 intelligence queries |
| `weak` | Owner/manager-settled from a worker suggestion | Treat as advisory until settled; settle by writing a correction (§12.3) referencing the original |
| `blank` | Auto-write when only completion evidence exists | Treat as fact-of-completion, not fact-of-quality |

The sanitization gate (§12.6 Layer 1) is unchanged. This column is orthogonal: it classifies the *quality* of evidence, not its *safety* to record.

---

## §16.6.A Kernel-composed handoff packet (R11 landing)

> **Status.** New sub-section at the head of §16.6, immediately before "Inputs, provided at dispatch time:". Names a first-class artifact type that exists today in §16.6 implicitly.

### 16.6.A Kernel-composed handoff packet

> **Added — 2026-08-30.** Evidence: R11 (kernel-composed handoff packet as named artifact), wave-2 ai-memory typed handoffs + munder-difflin routing convergence, wave-5 LoopX #138 strongest prior art (typed claims/leases/task boundaries/typed continuation). §16.5 already scopes child subagents to "bounded to its parent's task, budget, and lifetime"; this subsection makes the *interface* between parent and child explicit.

A **handoff packet** is the structured artifact the kernel composes when dispatching a worker whose task carries state from a previous worker in the same dispatch tree. The packet is the parent's "claim" of what the child should know; the child receives the packet as input, never reads parent state directly. The packet shape:

```yaml
handoff_packet:
  parent_dispatch_id: str
  child_role: str                  # role from §14's role table
  child_inputs:                    # §16.6 input fields 1, 2, 3, 5, 6, 7 scoped to this child
    task_spec: str
    worktree_path: str             # fresh per child unless the parent grants a lease
    model_alias: str
    memory_allow_list: [category]
    per_dispatch_instructions: str
    token_ceiling: int
  budget:                          # §4.2 limits, child-share
    tokens: int
    wall_clock_s: int
  leases:                          # bounded delegation of parent resources
    parent_worktree_path: str      # optional — child reads but does not write
    parent_memory_read_categories: [category]   # optional
  claims:                          # assertions the parent makes about the child
    invariant_preserved: [str]     # e.g. ['§16.6 self-modification forbidden']
  evidence_refs: [decision_id]     # parent decisions the child should treat as established
  continuation:                    # typed handoff back to parent
    success_schema: object
    failure_schema: object         # §16.6.B.1
    return_address: parent_dispatch_id
```

The packet is constructed by the kernel at dispatch time, not by the parent worker. The parent submits the handoff intent (§4.4 cross-department request shape, simplified for in-dispatch), and the kernel composes the packet from the parent's role config + the child's role config + the dispatch's accumulated state. This is the §4.4 rule applied to in-dispatch delegation: kernel-mediated only, never peer-to-peer.

The packet is logged as a `dispatch_event` (§16.10.7) with class `dispatch.handoff`. The §16.9 manager's `previousRun` argument (Codebuff #147 SDK reference, waves 5–6) is a degenerate case of this packet without the leases/claims/continuation fields.

---

## §14.7.A Incident-rule DSL (R13 landing)

> **Status.** New addendum to §14.7 (Security Department). Lands as `§14.7.A Incident-rule DSL` immediately after the existing role table.

### 14.7.A Incident-rule DSL

> **Added — 2026-08-30.** Evidence: R13 (incident-rule DSL), wave-2 spiderfoot YAML correlation rules, wave-3 K-Dense fail-closed analysis posture, wave-5 Codebuff/LoopX/provenance strengthenings. The existing §14.7 `incident_log` write path is structured (§12.2 fields); this subsection specifies the *declarative rule layer* that produces incidents from events.

The Security department's `incident_log` (existing) and the kernel's `dispatch_event` stream (§16.10.7) are the inputs. An incident-rule DSL compiles to SQL over these tables:

```yaml
- id: stale_skill_referenced
  when: |
    dispatch_event.class IN ('worker.tool_call') AND
    payload.tool = 'invoke_skill' AND
    EXISTS (SELECT 1 FROM skills s WHERE s.id = payload.skill_id AND s.stale_by < now())
  then:
    severity: warn
    actions: [suspend_skill, notify_owner]
  evidence_required: [skill_id, last_used_at, stale_by]

- id: cross_dept_acl_violation_attempt
  when: |
    dispatch_event.class IN ('worker.tool_call') AND
    payload.tool IN ('read_memory', 'write_memory') AND
    payload.memory_category NOT IN (SELECT unnest(allowed_categories) FROM dispatch_context WHERE dispatch_id = dispatch_event.dispatch_id)
  then:
    severity: critical
    actions: [escalate_to_manager, log_attempt, notify_owner]
```

Rules are declared in YAML, compiled to SQL by the kernel at registry load, and evaluated by the Automation department's `monitor` role (§14.6) on the event stream. The DSL is deliberately narrow — match → aggregate → typed risk — and does not generalize to arbitrary computation; complex reasoning about an incident stays the `audit_reviewer` role's job.

Each rule carries an `evidence_required` list specifying the field set the rule must populate before the `then` clause fires. A rule that fires without its evidence set is a rule-engine failure, not an incident — the failure itself is logged.

---

## §17.3 Below-Low tier extension (R15 + proposal #5 landing)

> **Status.** Extension to §17.3 (Product Intent, First-Run Experience & Model Packs → Model Pack Categories). Lands as a new sub-section after the existing "Per-tier performance reference and fast-follow triggers."

### 17.3.A Below-Low tier

> **Added — 2026-08-30.** Evidence: R15 (hardware-capability probe → generated compat matrix), wave-2 lemonade `backends` probe + local.ai setup benchmarks, wave-5 needle-2 #121 (28MB-RAM tool-caller) + yap #117 (zero-model Apple SpeechAnalyzer) + local.ai #69 + RF-DETR #118. The existing tier mapping (Low ≤ 8 GB / Mid 8–16 GB / High > 16 GB) covers discrete-GPU hardware; this subsection covers the devices below Low.

Devices below the Low tier — MCU-class, sub-4 GB RAM, no discrete GPU, budget phones — cannot run the §16.1 local inference stack at all. LastHarness does not exclude them from the product; instead, this subsection specifies the **probe-generated compat matrix** as the Step-0 primitive for picking a below-Low deployment.

**Probe.** Step 0's hardware scan gains a `compat_matrix` field derived from the device's reported capabilities (CPU arch, available RAM, NPU presence, OS-shipped model availability). The matrix is generated, not configured: each candidate capability (`llama.cpp`, `whisper.cpp`, OS-shipped speech, OS-shipped vision) is probed by attempting a 1-token test call (cf. Appendix A Step 1B for the inference-provider pattern). Failed probes are reported as `false`; successes as `true` with the resolved version.

**Compat matrix.** A Below-Low install runs only the capabilities its matrix reports `true` for. A device with no `llama.cpp` capability gets a *no-local-LLM* profile: it still runs LastHarness, but its department dispatches route through the owner's chosen inference provider (§17.2 Path A/B) — which, on a below-Low device, may itself be limited to small models. A device with `whisper.cpp` only gets voice capture, not text reasoning. A device with OS-shipped speech + vision gets the speech/vision surfaces but no general text.

**Compat matrix disclosure.** The compat matrix is shown to the owner during onboarding (Step 5 summary screen, §17.2), as `Installed capabilities: [...] / Not available: [...]` — not as a flat "your device can't run LastHarness" verdict. This is the same UI discipline that prevents a below-4 GB device from blocking onboarding (Appendix A Step 4 C.Failure Handling).

**Below-Low and the spec invariants.** Every §16.5/§16.6 invariant holds at below-Low: the kernel still runs, the dispatch contract still has machine-readable terminal status, the decision log is still append-only, and the §16.1 boundary still applies. What changes is the *model* in the dispatch — the device's available capability is what the alias resolves to (§16.1 `llama-swap`), not the install's GPU.

*Sandbox assertion (§11.3):* on a stubbed below-Low device profile, dispatch a Development task. PASS = dispatch succeeds via the owner's chosen inference provider, with the decision-log entry recording the actual model used (the provider model, not "no model available"). FAIL = dispatch fails on the local-routing assumption alone.

---

## §16.1 Single-artifact packaging contract (R16 landing)

> **Status.** New sub-section at the end of §16.1, immediately before "Local inference stack."

### 16.1.A Single-artifact packaging contract

> **Added — 2026-08-30.** Evidence: R16 (single-artifact packaging contract incl. two-tier backup semantics), wave-2 pocketbase #130 one-binary shape. The existing `installer/` is "an internal module inside the main LastHarness repo" — this subsection specifies the installable shape.

A **LastHarness install bundle** is a single artifact per platform — one `.exe` for Windows, one `.dmg` for macOS, one `.AppImage` / `.deb` / `.rpm` for Linux, plus a single bundled `.iso` for air-gapped installs. The bundle carries:

1. The kernel + departments + manager dispatch runtime (Python + dependencies).
2. The local inference stack (`llama-server` + `llama-swap`, §16.1).
3. The selected hardware-tier model pack (§17.3) — *or* a deferred-download path for manual tier selection post-install.
4. The Linux environment seed image (WSL2 distro tarball for Windows; containerization image for macOS; systemd-nspawn/bubblewrap image for Linux).
5. A **two-tier backup metadata block**:
   - **Tier 1 (auto):** SQLite database + key Vault entries, written at the user's data directory on every successful write. This is *the* backup that survives reinstalls of the app binary.
   - **Tier 2 (manual):** full install snapshot — model GGUFs, skills registry, kernel config — written to a user-chosen path on demand. Tier 2 is *not* automatic; auto-snapshotting tens of GB of model weights is rejected as wasteful.

The bundle is checksummed end-to-end (§5.4's department-package discipline extended to the installer). Two consecutive checksum failures surface a manual-retry UI; the user never sees a silent partial install.

This packaging contract is independent of the §16.1 isolation boundary — the boundary's *post-install* shape is unchanged. The contract specifies only the *delivery* shape: one file, verified, two backup tiers.

---

## Cross-reference summary

| Section | Lands at | Status | Origin evidence |
| --- | --- | --- | --- |
| §16.10 Owner Surfaces | new section | re-drafted | R1, R2, R7 recurrence (wave 1); wave 5/6 strengthenings |
| §16.10.7 Worker-event stream | §16.10 sub-section | new | wave-6 Codebuff #147 prior art |
| §14.5.A Skills Intake Lifecycle | §14.5 addendum | new | R5×8 (waves 1, 2, 3, 5) |
| §7.1.B Compaction Protocol | §7.1 addendum | new | R3×5 (waves 1, 3, 5) |
| §16.6.B Result Schemas v2 | §16.6 addendum | new | R6×3, R10×4, R12 (waves 1, 2, 3) |
| §16.6.A Kernel-composed handoff packet | §16.6 sub-section | new | R11 (waves 2, 5) |
| §12.2.A Evidence strength column | §12.2 paragraph | new | R12 (wave 2) |
| §14.7.A Incident-rule DSL | §14.7 addendum | new | R13 (wave 2) |
| §17.3.A Below-Low tier | §17.3 sub-section | new | R15, proposal #5 (waves 2, 5) |
| §16.1.A Single-artifact packaging | §16.1 sub-section | new | R16 (wave 2) |

**Total:** 10 spec changes proposed from the wave-1..6 dossier corpus. None of them alter any locked invariant (§16.1, §16.5, §16.9, §12.6). All are refinements or addenda that close named spec gaps.

**Not proposed** (recorded but not drafted):
- Department additions beyond the existing 6 (proposals #1–#5 from `11-department-blueprints.md` all land as §16.10 / §14.5 / §7.1 / §16.6 / §17.3 addenda, not as new departments — the "no fifth Team Memory department" precedent holds).
- "Meeting-Ops" and "Media Production" parked departments (wave-2 finding; awaiting owner demand and §16.4 fast-follow triggers respectively).
- User-install vs company-operated separation (wave-6 spec gap; needs §17.1 addendum — out of scope of this extraction PR).

---

*End of DRAFT. Review section-by-section; merge into `docs/archive/lastharness-v1-spec.md` after sign-off.*