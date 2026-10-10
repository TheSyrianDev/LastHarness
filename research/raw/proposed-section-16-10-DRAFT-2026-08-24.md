### 16.10 Owner Surfaces

> **Added — 2026-08-24.** Proposed as blueprint #1 of the feature-exploration program (`research/extraction/11-department-blueprints.md`) and adopted on recurrence evidence: approval gates before side-effectful actions appear in 6 of 127 surveyed tools, editable plan-before-execute phases in 5, owner visibility surfaces in 4+, with remote-control auth ladders concurring independently (full dossiers under `research/surveys/10-feature-dossiers/`). This section creates a **render-and-gate surface**. It creates no new authority, no standing process, no new transport, and no exception to any invariant above: §16.5's ephemeral-dispatch rule, §4.4's kernel mediation, §16.4's messenger exclusion, and §7.2's override mechanics all hold unchanged.

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

---

