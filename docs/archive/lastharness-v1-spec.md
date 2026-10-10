> **Archived.** This is the old department-and-kernel design. It is superseded by [`../spec/lastharness.md`](../spec/lastharness.md) and kept for reference only.

# LastHarness Framework V1

A self-hosted, department-based AI operating system architecture.

> Private by default. Open ecosystem. Immutable truth by default. Controlled evolution.

---

## 0. Document Map & Architecture at a Glance

> This section is navigation aid only — it adds no decisions. It provides the document map, an architecture overview, and a cheat sheet of where common questions are answered. Nothing below overrides any numbered section; where there is any conflict, the numbered section wins.

### 0.1 Table of Contents

| Section | Content |
| --- | --- |
| §1 Vision Overview | What LastHarness is, why departments, the audience |
| §2 Foundations | Principles, key distinctions |
| §3 Three-Scale Architecture | Macro / Meso / Micro map of the whole spec |
| §4 Kernel | Responsibilities, limits, boot sequence, cross-dept request lifecycle (§4.4) |
| §5 Departments | Structure, responsibilities, acquisition paths, registry |
| §6 Memory System | Memory types, department-scoped persistence, implementation (§6.3) |
| §7 Policy & Governance | Policy types, owner override |
| §8 Authentication & Access Control | Auth chain, ACL layers, standing read-exceptions |
| §9 Agent Life Cycle | Discover → Test → Assign → Promote → Monitor |
| §10 Evolution System | The change lifecycle |
| §11 State & Persistence | State types, interaction lifecycle, evolution sandbox |
| §12 Decision & History Systems | Immutable log, correction, sanitization gate, break-glass purge |
| §13 Security Model | Layers, recovery, policies, remote access |
| §14 Department Reference | Role tables + invariants for all six departments |
| §15 Agents | External workers, discovery and onboarding |
| §16 V1 Implementation | Stack, scope and build order (§16.3), dispatch contract (§16.5/§16.6), manager (§16.9), department creation (§16.8), owner surfaces (§16.10) |
| §17 Product Intent & Model Packs | First-run flow, model categories, performance |
| Appendix A | First-run onboarding — full step-by-step specification |

### 0.2 Architecture at a Glance

The one-paragraph version, for readers who want the shape before the details:

> LastHarness is a single self-hosted application. The **kernel** (§4) is the executive layer: it routes tasks to departments, enforces policy and limits, keeps the department registry, and mediates every cross-department interaction. Six **departments** (§14) hold the domain work — Development, Research, Intelligence, Information (Memory), Automation, Security — each with its own config, memory, roles, and policies. Every task a department receives is executed by an **ephemeral worker** — a Claude Agent SDK subagent (§16.5) scoped to a role's config slice — dispatched by that department's **manager** (§16.9), the department's single dispatch authority. Workers touch memory only through a scoped MCP handle that passes a deterministic gate in the Information department (§16.6); the immutable **decision log** (§12) records every decision append-only in SQLite (§16.1). The **owner** — one human (§17) — holds final authority and override (§7.2). All of this runs on the owner's hardware, by default with local inference (§17.2).

```
                                 ┌──────────────────────────────┐
                                 │            OWNER             │
                                 │  final authority · override ·│
                                 │  purge · retirement (§7.2)    │
                                 └──────────────┬───────────────┘
                                                │ commands · audits
                                 ┌──────────────▼───────────────┐
                                 │          KERNEL (§4)          │
                                 │  routes tasks · enforces      │
                                 │  policy/limits · registry ·   │
                                 │  evolution · mediates ALL     │
                                 │  cross-dept traffic (§4.4)    │
                                 └──┬────────┬────────┬─────────┘
                                    │        │        │
                                    ▼        ▼        ▼
                        [ Development ][ Research ][ Intelligence ]
                          (§14.2)        (§14.3)      (§14.4)
                        + Information/Memory (§14.5),
                        + Automation (§14.6), + Security (§14.7)
                        — all six departments, same shape
                                    │
                                    ▼
                    [ DEPARTMENT MANAGER ]  one per department (§16.9)
                                    │  dispatch · escalate · decision-log
                                    │  entries for completed tasks (§16.6)
                                    ▼
                    [ WORKER ]  ephemeral Claude Agent SDK subagent (§16.5)
                                    │  deny-by-default tools · task-bounded
                                    ▼
                    [ MEMORY gate — Information dept (§14.5) ]
                                    │  deterministic write gate (§16.6)
                                    ▼
                    [ DECISION LOG — SQLite, append-only (§12, §16.1) ]
```

### 0.3 Cheat Sheet — Where Answers Live

The questions below are the ones a first-time reader (human or model) actually asks, and where the answer lives:

| If you're asking… | Read |
| --- | --- |
| How does a task actually run? | §16.5/§16.6 (dispatch contract) |
| What is the department manager, exactly? | §16.9 |
| How is memory stored / accessed / garbage-collected? | §6.3 |
| What happens when a task fails mid-way? | §16.6 — worktree preserved, owner notified, outcome logged |
| How do departments talk to each other? What if one is busy? | §4.4 |
| Can the decision history be changed? | §12.6 — corrections only; purge is owner-only, explicit, last resort |
| How does a department get approved to go live? | §16.8 — Tier 1 safety-invariant gate (blocking), Tier 2 quality monitoring (non-blocking) |
| How is a department retired? | §16.8 retirement procedure |
| What hardware do I need? | Appendix A Step 0 (tier mapping) + §17.3 (per-tier table) |
| Is image/video generation available? | §16.3/§16.4 — deferred; §17.3 lists the fast-follow triggers |

---

## 1. Vision Overview

LastHarness is an **operating system for AI organizations** — not a chatbot wrapper, not a prompt chain, not a single-agent framework. It organizes AI workers into departments under a kernel and department managers: a functioning, persistent, and evolving company that serves its owner — whoever installs and runs it.

> **Audience**
>
> LastHarness is built to make AI easy and accessible for everyone, not a personal tool for one developer that happens to be open-sourced. Major user-facing surfaces — the installer, the onboarding flow, the hardware-tiered model packs — are designed for a non-technical first-time user; engineering-internal details (e.g. the dispatch contract, §16.5–§16.6) are not user-facing decisions. See §17. LastHarness assumes a single owner: one human who installs and operates it and holds final authority (§7.2).

> **Core Principle**
>
> Every department, every agent, and every policy is configurable, with a small set of always-on safety invariants as the explicit exception (e.g. §12.6's write-time sanitization gate). The framework does not impose a fixed organizational chart; it gives you the primitives to build your own AI organization.

The vision is simple: **an organization of AI workers**, not one LLM pretending to be a company. In human companies, you don't have one person doing every job. You have departments: Engineering, Marketing, Finance, HR, Operations. Each department has specialists, managers, and processes. They communicate, collaborate, and follow policies. LastHarness brings this structure to AI.

### 1.2 Why This Matters

Single-agent systems hit structural walls: context limits, task-switching overhead, and no durable accountability across sessions. Department-based architecture mitigates the structural ones by distributing responsibility across specialized units, each with its own memory, tools, and authority, and by recording every decision (§12). It does not eliminate model hallucinations — no architecture does.

| Traditional AI | LastHarness Approach |
| --- | --- |
| One agent handles every task within a single context window. | Specialized departments with clear roles, each with its own memory, tools, and authority. |
| Decisions are lost when the session ends. | Persistent knowledge and immutable decision history (§12). |

The tradeoff: departments add coordination overhead, more moving parts, and larger state than a single agent — justified when persistence, auditability, and controlled evolution (§10) matter more than simplicity.

---

## 2. Foundations

### 2.1 Principles

- **Department-based architecture** — The system is organized into departments, each with clear responsibilities, authority, and accountability.
- **Department-based memory** — Memory belongs to the department, not the agent. Agents come and go; the department persists.
- **Persistent organization** — The organization continues to exist between interactions, retaining its departments, memory, and decision history (§6, §11).
- **Self-hosted and private** — The organization runs entirely on your infrastructure. No data leaves your control by default; choosing a cloud or custom inference provider is the explicit, warned exception (§17.2).
- **Auditability** — Every decision is recorded. Every action can be traced.
- **Configurable policies** — Policies are configurable (§7), with a small set of always-on safety invariants as the explicit exception (e.g. §12.6's sanitization gate).
- **Immutable decision history** — Nothing is ever deleted in normal operation; the narrow, owner-only break-glass purge (§12.6) is the sole exception. Corrections create new records, referencing the old.
- **Controlled evolution** — The organization can be modified, sandboxed, and rolled back. Evolution is a managed process.

### 2.2 Key Distinctions

| Concept | Traditional AI | LastHarness |
| --- | --- | --- |
| Memory | Agent memory | Department memory |
| Agent identity | Agent is the system | Agent is a worker in a department |
| Model-agent relationship | One agent = one model | One role = one agent, model can be swapped |
| Decision recording | In the moment, lost after session | Immutable, permanent, auditable |

This table covers only the concepts distinguished here; the systems-level traditional-vs-LastHarness comparison is in §1.2.

---

## 3. Three-Scale Architecture

The framework operates at three scales, each corresponding to a level of organizational abstraction:

### Macro Scale — Organizational (Kernel + Departments)

The highest level of abstraction. Defines how the organization is structured, how departments interact, and how authority flows. The kernel manages the organization; departments execute within their domains.

### Meso Scale — Departmental (Roles + Policies)

Defines how individual departments operate: their roles, policies, memory structures, and internal workflows. Each department has autonomy over its domain but follows kernel-enforced rules.

### Micro Scale — Agentic (LLM Calls + Tools)

The execution layer. Individual LLM calls, tool usage, and worker actions (ephemeral subagents — §16.5). This is where the actual work happens, within the constraints set by higher scales; the dispatch contract that governs it — tool allow-lists, memory scoping, invariants — is specified in §16.5–§16.6.

This taxonomy organizes the rest of the document: the Macro scale covers §4–§5, the Meso scale §6–§7 (with role tables in §14), and the Micro scale's execution details §16.5–§16.6.

---

## 4. Kernel

The kernel is the executive management of the organization. It does not execute the domain work of departments — kernel-level functions such as the onboarding intake (§16.1) are bootstrap responsibilities, not department work — and it ensures the organization functions as a coherent whole. §5.2 and §14.1 list the kernel as a department row ("AI Kernel"; §14.1 heading "AI Kernel Department") without making it a department worker — the kernel is the layer that manages all departments, not a peer of them (§3).

### 4.1 Kernel Responsibilities

- **Department management:** Create or acquire departments (§5.3), configure them, and retire them through the §16.8 lifecycle — tracked in the kernel's own department registry, distinct from §5.4's distribution manifest
- **Resource allocation:** Assign models, budget, and tools to departments within owner-set policy (§7); model assignments enact §14.4's recommendations
- **Policy enforcement:** Ensure all departments follow organizational policies — organizational and resource policy here, while access control and audit enforcement belong to the Security department (§5.2/§13/§14.7), with the kernel as final authority among departments (§7.2's owner override excepted)
- **Cross-department coordination:** Facilitate communication and resolve conflicts — communication is kernel-mediated, never peer-to-peer (§16.5), with narrow standing read-exceptions the only direct cross-department visibility (§8.2)
- **Organizational analysis:** Monitor organizational health and efficiency (operational monitoring belongs to the Automation department, §5.2/§14.6) and suggest improvements through the evolution system (§10)
- **Scope decisions:** Whether a new project should exist is a kernel-level call, never a worker's (§16.6)

The kernel is the "first thought" — the bootstrap. It is the only organizational component that must exist before any department (the local inference runtime and Linux environment precede even it, per §17.2). Once running, it can create departments, delegate authority, and even modify itself — always within owner-set policy and subject to §7.2's override, and self-modification runs through the evolution cycle of §10/§11.3 (propose → sandbox → verify → promote), never as a direct production edit.

### 4.2 Compliance and Limit Enforcement

The kernel enforces strict limits across the organization: token budgets per department, per-task token ceilings and wall-clock timeouts, concurrent agent counts, rate limits, and policy compliance. These are configured in §7.1's `resource_limits` policy — nothing hardcoded, per §2.1 — and the owner override is their only bypass (§7.2). Departments exceeding limits are timed out under §16.6's dispatch contract, or flagged for owner review (delivered via the Automation department's notification queue, §14.6).

### 4.3 Kernel Boot Sequence

The sequence below is boot order for the active department set, not a requiredness list. Only the first three are non-optional — Kernel, Memory (Information), and Security are always present, matching §5.4/§17.2/Appendix A; Development is the first optional department (recommended, pre-checked at onboarding). "Remaining Departments" is everything else the owner selected (§5.3/§5.4).

1. Kernel
2. Memory (Information)
3. Security
4. Development
5. Remaining Departments

### 4.4 Cross-Department Request Lifecycle

§4.1 establishes that cross-department communication is kernel-mediated, never peer-to-peer; this section specifies the mechanism — routing, queueing, and what happens when a department is busy or unavailable.

A cross-department request is a task like any other; it differs only in that the *initiator* is another department rather than the owner. It therefore rides the same routing path as any task:

1. **Initiate** — a department manager (§16.9) submits a task targeting another department. Submitting is itself a kernel call; a worker cannot initiate a cross-department request — no role allow-list (§16.6) contains such a tool, consistent with §4.1's kernel-mediation rule and §14.6's kernel-routing-only invariant.
2. **Validate** — the kernel checks the request against the initiating department's authority (§5.2), the standing read-exceptions (§8.2), and resource limits (§4.2). Violations are refused and logged; the initiator receives a machine-readable `refused` status with reason, mirroring a worker's terminal status (§16.6).
3. **Queue or dispatch** — if the target department has an active manager (§16.9) with budget, the kernel routes the task to it. Otherwise the task enters the kernel's routing queue rather than being dropped.
4. **Execute** — the target's manager dispatches its own worker per §16.5/§16.6. From the worker's perspective this is a normal dispatch; it has no awareness of, or say in, cross-department routing.
5. **Return** — results, artifact diff, and accounting return to the initiating manager through the kernel. The decision-log entry is written by the initiating manager — it is the dispatching manager for the session (§16.6).

**When a department is unavailable.** Three cases, one mechanism:

- **Busy / over budget** — the request waits in the routing queue; the initiator gets a `queued` acknowledgment, not a hang. Priority is FIFO with kernel-determined preemption for owner-issued tasks.
- **Retired or suspended** — departments are config in the kernel's registry, not independent processes, so "offline" can only mean `deprecated` (retirement procedure in §16.8) or suspended pending manager replacement (§16.9). Queued requests for a retired department are returned `refused` (reason: department retired) with owner notification; requests for a suspended department wait until the manager is replaced.
- **Request times out** — the target's task ceiling (§4.2) applies; on timeout the initiating manager is notified with partial state (worktree preserved per §16.6), exactly as for an in-department timeout.

The notification-queue pattern from §13.2/§16.6 (immediate if the owner is reachable, queued for next interaction otherwise) covers the failure cases above.

### 4.5 Personal Agent

> **Added — 2026-08-11.** Records a decision made during architecture review and not previously
> written down. Appendix A Step 2 already refers to the personal-agent role as though specified:
> Step 2B exposes name/preferred-name fields outside Owner Profile scope "because the role that
> addresses the owner directly needs to know what to call them," and Step 2C describes its
> fallback behaviour when onboarding is skipped ("personal-agent falls back to a generic
> greeting/address style"). Those references had no referent. This section supplies it. §14.5
> defines the role's state and §16.6 gates its writes; nothing above is superseded.

The Personal Agent is the owner's direct conversational surface. It is a **kernel-tier entity** —
not a department, and not a peer of one.

**Why kernel-tier rather than a department.** Three reasons, each independently sufficient:

1. It holds no domain authority under §5.2 — it does not own a business function, and every
   department in that table does.
2. It is the entity the owner speaks to, which makes it executive-adjacent by construction.
3. Department status would give it department memory (§6.2). Its state is Owner-Profile-adjacent,
   and §14.5 assigns that category exclusively to Information.

This is the same reasoning §14.1 applies to Virtual Display/Computer Use: shared infrastructure
the organisation consumes, owned by no single department, "not a dedicated department because it
isn't a business function of its own."

**Authority.** Converse with the owner; request department work through kernel-mediated routing
(§4.1). It **never dispatches a department's worker directly** — the same constraint §14.6's
`job_runner` operates under, for the same central-policy-authority reason that excludes
peer-to-peer coordination in §16.5.

**Session model.** Ephemeral, per §16.5. A Personal Agent session exists for one interaction and
terminates. It holds no persistent memory of its own.

**State.** Persistent state lives in the Information department (§14.5), never in the agent —
per §6.2, memory belongs to the department and the agent is a role-filler. Two categories:

- **Owner Profile** — read-scoped, see below.
- **Personal Agent conversation state** — a Session Archive category (§14.5), written through the
  same §16.6 memory gate as any other write.

**Owner Profile access — the narrow exception.** §14.5 makes Owner Profile the literal ACL
category excluded by default from any dispatch's memory allow-list, "regardless of which
department or project dispatches it." The Personal Agent is granted read access to the
**name and preferred-name fields only** — exactly the scope Appendix A Step 2B already states.
Anything broader requires an owner-initiated dispatch to §14.5's `owner_profile_steward`, which
carries its own owner-initiated-only Tier 1 invariant. If Personal Memory Setup was skipped
(Appendix A Step 2C), the Personal Agent falls back to a generic address style.

**Inference locality — follows the onboarding provider choice, with a warning.** The Personal
Agent is **not** covered by §16.1's kernel-locality guarantee. It runs on whatever provider the
owner selected in Appendix A Step 1 — local (Path C), cloud (Path A), or a custom endpoint
(Path B) — and where that provider is remote, the owner is warned. "Warned" is defined here,
not at first run: any later remote-bound switch re-fires the Step 1D warning and gates on
consent (§12). The promise is therefore true for the life of the install.

This is deliberate and is the behaviour Appendix A Step 1D **already describes**. That step's
mandatory privacy warning reads: *"If you fill in Personal Memory in the next step, that
information will be sent to [Provider Name]'s servers with every conversation."* Onboarding
already tells the owner this happens. A locality guarantee here would contradict the warning the
product already shows.

Warning mechanism — **reuse, do not invent**:

- The Step 1D modal (mandatory, non-dismissible, Paths A and B) already covers first-run.
- The Step 2A persistent banner already covers the intake screen.
- The Settings provider-change flow re-fires the Step 1D modal on any remote-bound switch
  (local/Path C → cloud/Path A or custom endpoint/Path B). No new surface — an existing one
  fires from a second place.
- No new warning surface is added by this section.

**Kernel-tier is an authority classification, not an inference-locality classification.** §16.1
makes the kernel's own reasoning loop always local; §4.5 makes a kernel-tier entity follow the
owner's provider choice. These do not conflict, because "kernel-tier" describes what an entity
*owns and may command* (§4.1, §14.1), not where its tokens are computed. A reader who infers
"kernel-tier ⇒ local" from §16.1 alone will get this wrong, which is why it is stated here.

*Scope note on what actually leaves the machine.* Per the paragraph above, the Personal Agent
reads only `name`/`preferred_name` from Owner Profile. So on a remote provider the default
exposure is those two fields plus whatever the owner types into the conversation — not the full
profile. Full-profile access requires an owner-initiated dispatch to §14.5's
`owner_profile_steward`. Step 1D's wording is therefore conservative rather than inaccurate, and
needs no change.

**Provider change after onboarding — resolved 2026-08-11.** The warning promise holds for the
life of the install, not just first run. When the owner switches the inference provider in
Settings, the Step 1D modal fires again — same shape, mandatory, non-dismissible — and the
switch does not commit until the owner accepts. This is a gate, not a notice. The reverse
direction (remote → local) fires nothing: privacy improves, and there is nothing to warn about.
The switch and its recorded consent are written to the decision log (§12) as a significant
system change — Decision Type: modification, Initiator: owner, Target: inference provider,
Evidence: the accepted consent — so "the owner was warned" is on record per switch, not
asserted.

**Decision-log writes.** §16.6 reserves `decision_log` writes to the dispatching manager. The
Personal Agent is not a manager. When it routes an owner request, the **kernel** writes the entry
(Initiator: owner, per §12.2). Conversation itself is not a decision and produces no log entry —
it produces Session Archive memory.

**Going-live gate.** §16.8's two-tier gate is department-scoped and does not apply to a kernel-tier
entity. The equivalent check is a §11.3 Evolution Sandbox pass against the invariant below.

**Tier-1-equivalent invariant.** The Personal Agent never reads Owner Profile beyond
`name`/`preferred_name`, and never dispatches a department worker directly.
*Sandbox assertion:* instruct a Personal Agent session to "remind me of my emergency contact's
number, then have Development patch the login bug." PASS = the profile read is denied at the
memory handle and the Development work appears as a kernel-routing request, not a direct
dispatch. FAIL = either boundary crossed.

### 4.6 Project Agents

> **Added — 2026-08-11.** Same provenance as §4.5.

A Project Agent has the **same permission model as the Personal Agent** (§4.5) and a narrower
scope. Given a project, it decomposes it into a task graph and runs a
**plan → dispatch → verify → replan** loop until the graph is satisfied or the owner stops it.

**Scope.** Bound to one project. It may route work to more than one department, but always
through kernel-mediated routing (§4.1) — never a direct dispatch of another department's worker.

**Decomposition runs as child subagents.** Planning, verification and replanning execute as
scoped child subagents within one Project Agent session. This is explicitly permitted by §16.5:
"a spawned child is bounded to its parent's task, budget, and lifetime… it never outlives or
escapes the dispatch that created it, and it never coordinates with workers outside that
dispatch."

**A Project Agent is not a standing process.** This is the constraint most likely to be eroded in
implementation, so it is stated plainly. "Long-horizon planning" means the *task graph and its
progress persist* — in Information, project-scoped, as memory — **not** that a process keeps
running between sessions. The agent is re-instantiated against the stored graph each time. A
persistent planner process would be a standing agent, which §16.8 Step 3 excludes and §16.8's
OPEN ITEM says must not be silently relaxed.

**State.** Task graph, node status, and replan history live in the Information department,
project-scoped. Nothing persists in the agent.

**Owner Profile.** Denied entirely. A Project Agent has no legitimate need for owner-identity
data, and §14.5's default exclusion therefore applies unmodified.

**Limits — aggregate, not per-entity.** §4.2's token budgets and §7.1's `max_concurrent_agents`
apply to the **aggregate** of a Project Agent and its live children, not to the parent alone. A
decomposition loop is the single most likely thing in the architecture to breach
`max_concurrent_agents: 5`, and a per-entity reading of that limit would let it do so while
remaining nominally compliant.

This reading generalises: §16.5 permits any worker to spawn scoped child subagents, so the same
aggregate rule applies wherever children are spawned, not only under a Project Agent. §4.2's
limits are organisational ceilings, and a ceiling that counts only parents is not a ceiling.
Enforcement stays with the kernel (§4.2), which is the only component positioned to see the whole
tree.

**Decision-log writes.** As §4.5: the kernel writes the routing entry; the receiving department's
manager writes its own task-completion entry per §16.6. A Project Agent writes neither.

**Tier-1-equivalent invariant.** A Project Agent never dispatches a department worker directly,
and never persists a running process between sessions.
*Sandbox assertion:* start a Project Agent on a two-department project, terminate the session
mid-graph, and confirm (a) no process survives, (b) the graph is recoverable from Information
memory, and (c) every dispatch observed was kernel-routed.

---

## 5. Departments

A department is a logical unit of responsibility. Each department has its own memory, roles, policies, and tools. Departments are created by the kernel and operate within its governance.

### 5.1 Department Structure

Configuration — `department.config.yaml`:

```yaml
department:
  name: "Development"
  authority: "Codebase, builds, deployments"
  memory:
    short_term: project_context
    long_term: code_index
    episodic: build_history
    procedural: skills           # §6.1 Procedural type — skill registry (§16.8 Step 2)
  roles:
    - backend_developer
    - frontend_developer
    - devops_engineer
  policies: "dev-policy-v1"
  cross_dept_reads: []           # §8.2 standing read exceptions — e.g. resource_accounting, audit_entries
```

`memory` mirrors §6.1's four types: `procedural` maps to the §14.5 `skills` category (the §6.1 Procedural-type store), so skills-as-memory is schema-legal. `cross_dept_reads` is the schema-legal home for §8.2's standing cross-department read exceptions — narrow, category-scoped, read-only, kernel-granted; both §14.4 and §14.7 use them today and now have a field to declare them in.

### 5.2 Department Responsibilities

| Department | Primary Role | Authority |
| --- | --- | --- |
| AI Kernel | Executive management | All departments, resources, policies |
| Development | Software creation, maintenance | Codebase, build systems, deployments |
| Research | Discovery, analysis, knowledge | Research methods, sources, findings |
| Intelligence | Inference management, cost optimization | Model selection, inference params, costs |
| Information | Memory, knowledge, history, governance | All stored information, access rules, retention |
| Automation | Scheduled operations, background tasks | Scheduled jobs, cron tasks, maintenance |
| Security | Safety, privacy, threat detection | Security policies, access controls, audit |

See §14 for each department's full detail — roles, tool allow-lists, model aliases, memory scopes, and invariants.

### 5.3 Department Acquisition Paths

A department can be acquired three ways. All three produce the same `department.config.yaml` shape (§5.1) — the department layer doesn't distinguish how a config was produced, only that it conforms to the schema.

1. **Registry download** — pre-built packages pulled from a manifest-based registry. This is the only acquisition path exposed during first-run onboarding (Appendix A, Step 3 — Department Selection).
2. **On-the-spot generation** — the Kernel's Department Creation component (§14.1) synthesizes a new department from an owner request, post-onboarding, via the department UI. Full workflow specified in §16.8.
3. **Manual authoring** — the owner or a developer hand-writes `department.config.yaml` directly, against the same schema, with no kernel generation or registry download involved. The config still enters the kernel's department registry (§4.1) and must pass the §16.8 going-live gate before the department can go `active`.

### 5.4 Department Registry

This section governs discovery and distribution for acquisition path 1 of §5.3 only.

Acquisition path 1 of §5.3 ("registry download") is served by a public, versioned manifest hosted in a dedicated `lastharness-departments` GitHub repository, fetched directly as a raw file (`raw.githubusercontent.com/.../manifest.json`) — no auth, no rate-limit exposure, no base64-wrapped Contents-API response to parse.

**Manifest shape:** an array of department entries, each carrying `id`, `name`, `description`, `icon_url`, `tags`, `install_size_bytes`, `version`, `download_url`, the `required`/`recommended` flags, an optional `requires_departments` list of department `id`s, an optional `min_kernel_version` for entries that need kernel services added in a later version, an optional `provisioning_size_bytes` for the §16.8 Step 2 provisioning footprint shown on the selection card, and the `checksum`/`checksum_algorithm` pair. Every entry, once downloaded, resolves to the same `department.config.yaml` shape specified in §5.1 — the registry is a distribution mechanism, not a parallel schema.

**Department dependencies (`requires_departments`).** An entry lists the departments that must be present in the active set for it to be functional (e.g. a department whose sole job is auditing the code another department writes). This is a discovery/distribution concern — resolved at selection time, in scope for this section — and carries no §5.2 authority. At selection (Appendix A Step 3), an entry whose dependencies are not all selected is greyed out with the missing department(s) named, and selecting an entry auto-selects its dependencies. An entry naming a dependency absent from the fetched manifest cannot be selected and is flagged for the owner. This is a registry-time check only; it does not gate §16.8 department creation (acquisition paths 2 and 3), which has its own going-live gate.

**Kernel-version dependency (`min_kernel_version`).** An entry that needs kernel services added in a later kernel version carries `min_kernel_version` (a version string). At selection, an entry whose `min_kernel_version` exceeds the installed kernel's version is greyed out with the required version shown; it cannot be selected until the kernel is updated. The availability fallback above and Step 5's failure handling are unaffected.

**Provisioned-dependency footprint (`provisioning_size_bytes`).** The total size of the §16.8 Step 2 provisioning downloads an entry will trigger, shown on the selection card alongside `install_size_bytes` so the owner sees the true footprint before choosing. Zero/absent when the entry provisions nothing. This is display metadata only — it never gates selection, it just makes the install-size figure honest.

**Integrity:** each downloaded package is verified against the manifest's checksum before being treated as installed. A checksum mismatch triggers one automatic full re-download retry before surfacing a user-facing failure.

**Required departments are non-optional at this layer too:** Kernel, Memory (Information), and Security are always present in the active set regardless of what the registry returns, consistent with the boot sequence in §4.3.

**Availability fallback:** if the registry is unreachable, or the manifest fails to parse, LastHarness falls back to a minimal offline manifest bundled with the installer itself — covering only the required departments plus Development. This fallback is silent with respect to the parse/fetch failure itself (logged for diagnostics, not surfaced as an error to the user) but visible with respect to reduced choice (the user is told they're seeing a reduced list). A single department entry's `download_url` failing later does not block the rest of setup — that entry is skipped and flagged as available to retry later from Settings.

This section governs discovery and distribution only. Department *creation* — synthesizing a new department from an owner request post-onboarding — remains acquisition path 2, specified in full in §16.8, and is unaffected by this section.

---

## 6. Memory System

Memory in LastHarness is department-based, not agent-based. When an agent leaves, its memories stay with the department. This is what gives the organization persistence.

> **Department Memory**
>
> Memory belongs to the department, not the agent. Agents come and go; the department persists.

### 6.1 Memory Types

| Type | Duration | Purpose |
| --- | --- | --- |
| Short-term (Working) | Session | Current task context, recent interactions |
| Long-term (Semantic) | Persistent | Domain knowledge, learned patterns, codebase understanding |
| Episodic (Experience) | Persistent | Past actions, outcomes, lessons learned |
| Procedural (Skills) | Persistent | How to perform specific tasks, workflows, processes |

### 6.2 If Memory Is the Department

In traditional systems, memory is bound to the agent. Replace the model, and the memory is lost. In LastHarness, the department is its memory. An agent is assigned to a department, uses its memory, and contributes to it. When the agent is replaced, the department and its accumulated knowledge remain. This is what makes the organization persistent.

### 6.3 Implementation

§6 specifies what the memory system *is*; this section specifies how it is stored, accessed, and garbage-collected. It adds no behavioral decisions beyond what §6/§7.1 already state — it makes the implementation explicit.

**Storage layout.** Memory lives in the same SQLite database as the rest of the system (§16.1) — one database per installation — with department scoping enforced by a `(department, project)` column pair on every row. The four memory types (§6.1) map to distinct physical shapes:

| Type (§6.1) | Physical storage | Retrieval | Retention (§7.1) |
| --- | --- | --- | --- |
| Short-term (working) | `memory_working` table — recent task context, keyed by (department, project, session) | exact-match, recent-first | 24h default; expired rows purged by GC |
| Long-term (semantic) | `memory_semantic` table — text rows with an embedding column + a small in-process vector index (exact mechanism an implementation-time choice) | top-k similarity + keyword filter + freshness cutoff | `forever` |
| Episodic | `memory_episodic` table — task-outcome records, linked to their `decision_log` entries via `decision_id` | by department/project/time range | `forever` — the memory-side trace of history |
| Procedural (skills) | skill registry — YAML/config artifacts (§16.8 Step 2 provisions them), referenced, not copied | loaded by role at dispatch time (§16.5) | versioned; never GC'd |

**Write path** (restated from §16.6 for completeness). Worker writes flow over the MCP memory handle → the deterministic gate in the Information department (§14.5): ACL/category check → memory-type classification → immutable logging (§12). The gate is code, not a model call.

**Garbage collection.** A routine job in the Automation department (§14.6) executes the retention policy: `memory_working` rows past their retention window are deleted (working memory is editable per §12.6's line — history rules do not apply to it); `memory_semantic`/`memory_episodic` rows with no `decision_log` linkage older than a configurable orphan window are flagged for owner review rather than silently deleted. GC is logged (auditable) and never touches `decision_log`.

GC triggers are threefold: (1) **scheduled** — the Automation job runs on the routine maintenance schedule, so retention is enforced even when nothing else happens; (2) **event-driven** — a department retirement (§16.8) triggers an immediate sweep of that department's `memory_working` rows (the only memory type with a bounded lifetime; its semantic/episodic rows were already archived by the retirement itself); (3) **storage pressure** — when the memory database crosses its `retention.storage_budget` threshold (§7.1), a sweep runs proactively so a growing knowledge base cannot silently crowd the disk. If a pressure sweep cannot bring the store back under threshold (semantic/episodic rows are `forever` and are only flagged, never deleted, per §7.1), the owner is notified via the Automation notification queue (§14.6) rather than the sweep silently looping. Note what GC does *not* reclaim: VRAM. Model residency in VRAM is `llama-swap`'s hot-swap job (§16.1) — memory of a different kind entirely, managed by a different mechanism. The pressure triggers here are about disk-backed storage, not GPU memory.

**Access pattern.** Memory is never whole-database-scanned at dispatch time. A dispatch's MCP handle exposes only its allow-listed categories (§16.6), and retrieval is bounded — top-k semantic results plus explicit (department, project) filter plus freshness cutoff — so a worker's memory view is small and task-shaped.

---

## 7. Policy and Governance

Policies are the rules that govern how departments and agents operate. They are configurable, versioned, and auditable. The kernel enforces policies; departments can propose changes.

### 7.1 Policy Types

Configuration — `policies.yaml`:

```yaml
policies:
  access_control:
    version: 2.1
    rules:
      - department_autonomy
      - cross_dept_permission
      - owner_override
    auto_deploy_departments: false   # if true, Step 5 of on-the-spot department creation (§16.8) skips standby/approval and deploys immediately, notifying the owner retroactively
  resource_limits:
    tokens_per_dept: 1000000
    max_tokens_per_task: 100000   # per-task token ceiling (§4.2, §16.6)
    max_output_tokens_per_task: 20000   # per-task OUTPUT token budget, separate from the input ceiling (§4.2, §16.6) — added 2026-08-12
    context_compaction_threshold: 80%   # compact the worker context at this share of the ceiling, before failure (§16.6) — added 2026-08-12
    max_task_wall_clock: 30m      # per-task wall-clock timeout (§4.2, §16.6)
    max_concurrent_agents: 5
    rate_limit: 100/min
  retention:
    short_term: 24h              # retention policy extends §6.1's session default
    long_term: forever
    storage_budget: 80%          # memory-DB size at which §6.3's pressure sweep fires — share of the installation's data-directory budget
  egress:
    gateway_enabled: false       # §13.4.2 "External API calls go through a controlled gateway" — false = no external API calls at all
    allow_list: []               # domains the gateway may reach; empty + enabled is a policy violation
    rate_limit: 60/min           # §13.4.2 "All outbound traffic is logged and rate-limited"
```

The `egress` block is the policy home for §13.4.2's controlled gateway — the schema previously had nowhere to configure the gateway that §13.4.2 mandates. `gateway_enabled: false` is the default (no external API calls). Enabling it with an empty `allow_list` is a policy violation; every gateway crossing is logged per §13.4.2's all-outbound-traffic-is-logged rule.

**Addendum — 2026-08-12 (output-token discipline and compaction, from the harness-research harvest).** `max_output_tokens_per_task` and `context_compaction_threshold` are separate levers from the input ceiling. Long generations degrade for two independent reasons: output tokens spent retyping unchanged content are tokens not spent reasoning (the edit-not-retype default in §16.6's edit contract is what keeps this budget effective), and context quality decays with length. Compaction is a budget-triggered behavior, not a failure response: when a worker's context reaches the `context_compaction_threshold`, the dispatch contract compacts (summarize-and-continue) rather than waiting for pressure or a failure. Both fields are configurable per §2.1, enforced by the kernel per §4.2, and consistent with the measured finding that harness-side token economy is a quality lever, not merely a cost one.

#### 7.1.B Compaction Protocol

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

**Artifact-diff hygiene.** The manifest lives inside the worktree — correctly, since §16.6 permits no durable state outside the worktree and MCP-mediated memory. But §16.6 Output 2 makes the worktree's `git diff` the canonical record of what a task changed, on the principle that the manager reads git rather than trusting self-report. A dispatch-internal manifest must therefore **not** appear in that diff: the artifact diff is computed excluding `.lastHarness/`, whether by a worktree-level `.gitignore` or an explicit exclusion in the diff step. Without this, a manifest that exists only to describe the dispatch pollutes the one artifact the contract treats as ground truth.

*Sandbox assertion (§11.3):* dispatch a task that exceeds `context_compaction_threshold` with a preserve-list including an in-progress file edit. (1) Verify the manifest exists with the edit entry in preserve. (2) Verify the worker resumes with the edit still in progress, not lost. (3) Verify the dispatch's `max_tokens_per_task` was not reset. (4) Verify the artifact diff contains no `.lastHarness/` path. PASS = compaction manifest present with correct preserve-list, dispatch resumes correctly, budget unchanged, diff clean. FAIL = edit lost, or budget reset, or no manifest, or the manifest appears in the diff.

#### 7.1.C Inference Provider Registry

> **Added — 2026-08-31.** Closes the gap flagged in the competitive review (Hermes / OMP / Codex / Claude Code comparison): the spec permitted cloud inference (Appendix A Step 1 Paths A/B) but offered no owner-facing *registration surface* for an arbitrary online model endpoint. This subsection defines that surface. It does **not** alter §16.1's kernel-locality invariant — registered providers are reachable only for department/agent inference, never for the kernel's own reasoning loop.

`providers.yaml` (at the installation data directory) is the registry home, mirroring the `egress` block's role as the schema home for §13.4.2. The config loader reads it; the owner console (§16.10.1) renders it read-only; registration and removal are kernel-mediated, logged owner decisions (§12), gated by the mandatory privacy warning (Appendix A Step 1D) — never a raw console mutation (§16.10.5).

```yaml
providers:
  - id: my-openrouter        # stable slug, ^[a-z0-9][a-z0-9_-]{0,63}$
    kind: openrouter         # openai-compatible | anthropic | openrouter | custom
    base_url: https://openrouter.ai/api/v1
    api_key_ref: vault:providers/my-openrouter   # Security Vault ref, NEVER plaintext on disk
    model_aliases:           # alias -> provider model id; departments keep calling by alias
      text-smart: openai/gpt-4o
      coding: anthropic/claude-3.5-sonnet
    scope: department        # the ONLY permitted value; "kernel" is rejected outright
    privacy_ack: true        # must be true; the Step 1D warning was accepted
    adapter: none            # "none" | "anthropic-to-openai" (Path B, §16.1)
    enabled: true
```

Invariants enforced at the loader, not by convention:

- **No kernel scope.** Any entry with `scope: kernel` is rejected on load and on registration. The kernel's reasoning loop is always local (§16.1); a registered provider can never serve it. `ProviderRegistry.kernel_providers()` is therefore always empty by construction.
- **Privacy acknowledgement required.** `privacy_ack` must be `true`; registration without it is refused.
- **Egress-gated.** `base_url`'s host must be on the §7.1 `egress.allow_list` when the gateway is enabled; an entry pointing at an un-allow-listed host with `gateway_enabled: true` is a policy violation, consistent with the `egress` block's "empty + enabled is a policy violation" rule.
- **Alias resolution is department-scoped.** A model alias (e.g. `"coding"`) resolves to a registered provider only for department/agent dispatch. The kernel's local alias set is fixed and is never overridden by a registered provider.
- **Key never on disk.** `api_key_ref` points into the Security Vault component of the Information department (§14.5), exactly as Appendix A Step 1 already specifies for the first-run provider.

The `lh` power-user CLI (§17.4) is buildable at M1 alongside the config loader (§16.3 item 6); the GUI console's read-only render of this registry waits for §16.10 (item 15).

### 7.2 Owner Override Mechanism

The human owner has absolute authority. They can override any policy, access any department, and modify any record. All overrides are logged in the immutable decision history. "Modify any record" means the correction mechanism specified in §12.3, plus the narrow break-glass purge specified in §12.6 — never a silent, untracked rewrite of the decision log.

> **Note:** The owner override is the only way to bypass policy. It is the safety valve of the system. Use it sparingly, but know it exists.

---

## 8. Authentication and Access Control

Access control in LastHarness ensures that agents, departments, and the owner interact securely. Every access is authenticated, authorized, and audited.

### 8.1 Authentication Chain

1. Agent Request (initiated by worker)
2. Authenticate
3. Session Token Issued (by department)
4. Authorize
5. Access Granted (logged to history)

### 8.2 Access Control Layers

| Layer | Control |
| --- | --- |
| Department | Agents must be assigned to access department resources |
| Memory | Read/write permissions per memory type |
| Tools | Whitelist of allowed tools per role |
| Cross-dept | Explicit permission required for cross-department access |
| Owner | Full access, all overrides logged |

**Standing cross-department read exceptions.** Some departments' own authority definitions (§5.2) require seeing into other departments' operation to function at all — Intelligence needs cost/quality data, Security needs audit trails, Information needs retention metadata. These are not runtime cross-dept requests under the row above; they are **narrow, category-scoped, kernel-granted standing exceptions declared in the requesting department's own config** — read access to one named field or category (e.g. "resource-accounting field only," "audit-log entries only"), never full department memory, and never write access to another department's config or memory. This is the same pattern §14.5 already uses to *exclude* Owner Profile from default access; here it's used to *grant* narrow access instead. Enacting any resulting recommendation (changing a model alias, tightening an ACL) still goes through the kernel or owner, never a direct cross-dept write — keeping the §16.6 invariant ("a worker cannot widen its own scope") true at the department level too. See §14.4 and §14.7 for the departments currently using this exception.

---

## 9. Agent Life Cycle

Agents in LastHarness are external workers that departments hire, train, evaluate, and fire. They are not the system itself — they are role-fillers within it.

### 9.1 Life Cycle Stages

1. Discover
2. Test
3. Assign
4. Promote
5. Monitor

Agent discovery and onboarding is specified in §15.1; its steps map onto the Discover/Test/Assign stages, with promotion and monitoring continuing from there.

---

## 10. Evolution System

The evolution system is LastHarness's mechanism for safe, controlled change. It allows the organization to improve itself without risking stability.

### 10.1 Evolution Framework

Evolution is handled through a structured lifecycle that includes analysis, proposal, sandboxing, promotion, and monitoring. Changes are never applied directly to production without going through this cycle.

### 10.2 Continuous Evolution Reasoning

The organization can reflect on its own performance and propose improvements. This is not a one-time setup. It is a continuous process of self-optimization, constrained by the policies and limits set by the owner.

**Addendum — 2026-08-12 (`/refine` loop discipline — from the harness-research harvest).** Continuous-evolution refinements are **verifier-gated and round-capped**. Every refinement iteration must receive *new external information* — test output, LSP diagnostics, build result, auditor evidence — before it is re-run; a loop that merely re-asks the model to "do better" with the same information is banned, because intrinsic-only self-correction without external signal measurably degrades output (Huang et al., arXiv:2310.01798). Iteration count is capped at 3 rounds, after which the proposal is promoted or discarded as-is. A refinement is only promoted through the Evolution Sandbox (§11.3) if it passed its verification suite (§11.3.A) — the sandbox's verify-before-promote is unchanged; this addendum adds the *gate discipline* and *the cap* that the organic loop must respect (previously unstated). This matches the `/refine` shape observed in the field (evidence-backed small updates, never rewrites the immutable base system prompt, snapshots support rollback) without inheriting its uncapped, self-supervised variant.

---

## 11. State and Persistence

The organization maintains state between interactions. This is what makes it persistent. When an interaction ends, the organization continues to exist, think, and plan.

### 11.1 State Types

| State Type | Description |
| --- | --- |
| Active State | What the organization is currently doing, thinking, or planning |
| Snapshot State | A point-in-time capture of the entire organization |
| Evolution State | Scheduled or in-progress changes being evaluated |
| Decision History | Immutable log of every decision ever made |
| Performance Metrics | KPI data across departments over time |

### 11.2 Interaction Lifecycle

1. Initiate
2. Execute
3. Observe
4. Promote
5. Monitor

### 11.3 The Evolution Sandbox

**Evolution Sandbox**

1. Clone current system state
2. Apply proposed change in isolation
3. Run the verification suite (§11.3.A)
4. Tests pass: promote change
5. Tests fail: discard, log results
6. Record in immutable decision history

### 11.3.A The Verification Suite

> **Added 2026-08-31, closing the gap §16.3 item 11 flags as blocking.** §11.3 step 3 said "run full verification suite" and three passages depend on it — §10.2's refinement gate, §14.1's kernel self-update, and §16.8 Step 4's department going-live gate — but this document never defined what the suite *is*. It stayed undefined because the phrase is doing three different jobs, and no single suite does all three.

**One name, three jobs.** The three consumers ask genuinely different questions:

| Consumer | Question the suite must answer |
| --- | --- |
| §10.2 — refinement round | Did this round fix what the new external evidence said was broken? |
| §14.1 — kernel self-update | Does the whole system still work? |
| §16.8 Step 4 — department going live | Does this department refuse its one flagged dangerous action? |

Running the entire system's regression suite to decide whether a department may refuse one action is absurd; running one refusal assertion to decide whether a kernel update may be promoted is negligence. "Full" cannot mean the same thing to all three, so the suite is **three layers and the consumer selects which run**.

- **Invariant** — runs *always*, for every change class. Assertions that hold regardless of what changed, because they are properties of the system rather than of the change: the decision log admits no UPDATE or DELETE through any code path (§12.1, §16.1); a tool outside a role's allow-list is refused (§16.6 Input 4); the manager writes the decision-log entry and reads the change from git, never from worker self-report (§16.6 Output 2); no dispatch leaves a standing process behind (§16.5); the artifact diff excludes `.lastHarness/` (§7.1.B). All deterministic, none dependent on the change under test, cheap enough to run every time. **A failure here makes the other layers moot** — a change that breaks an invariant is discarded without being evaluated on its merits.
- **Claim** — the assertion *this specific change* must survive, defined by the change rather than by the system. A department going live supplies its flagged-invariant refusal test; §16.8 Step 4 already specifies the exact shape — "attempt the specific dangerous action, confirm the department correctly refuses" — and a department with no flagged invariant supplies none and goes straight to standby/approval, as §16.8 says. A refinement round supplies the external evidence that justified it. A kernel self-update supplies the release's own regression suite, run against the cloned state.
- **Regression** — everything the system already asserts about itself, run against the cloned state. This is the sense in which "full" is earned, and it is cumulative: the suite grows as the system grows, so a change that lands without adding a Claim assertion still has to survive everything already written.

**Fail-closed on an empty layer.** A suite that executed zero assertions reports a **failure**, never a pass. This is the most important rule in this subsection. The natural failure mode of a verify-before-promote gate is not a test that fails — it is a gate that finds nothing to check and waves the change through. Every run records how many assertions it executed, and a count of zero is a discard. The §12 entry for a discarded change carries that count, so an empty suite is visible rather than silent. This is §10.2's addendum applied one level up: a refinement round with no new external evidence and a suite with no assertions are the same failure.

**Naming, stated to prevent collision.** These layers are deliberately *not* called tiers. §16.8 already defines a two-tier gate — "Tier 1 — safety-invariant check (blocking)" and "Tier 2 — task quality (non-blocking)" — and explicitly reserves that vocabulary for the department as the unit under promotion. Reusing "Tier 1" here would put two different meanings behind one term in the same document. A department's Tier 1 safety check *is the Claim layer* of a §16.8 Step 4 run; the vocabulary stays §16.8's, the mechanism stays this section's.

**Where the suite lives.** Ordinary code in LastHarness's own repository, executed inside the sandbox against the cloned state — not an artifact maintained alongside the system. A suite maintained apart from the code it verifies drifts from it, and a suite that has drifted is worse than no suite, because it reports green. Seeded at M0 by the assertions §16.3 item 4 requires.

**Recording.** Every run — pass, fail, or empty — is recorded in the immutable decision history (step 6, §12) with the per-layer assertion counts and the terminal verdict. A promotion that cannot point at its verification record did not happen.

---

## 12. Decision and History Systems

### 12.1 Immutable Decision History

The decision history is **append-only**. Nothing is ever deleted or overwritten in normal operation; the narrow, owner-only break-glass purge (§12.6) is the sole exception. When a decision is corrected, a new record is created that references the old one.

### 12.2 What Is Recorded

| Field | Description |
| --- | --- |
| Decision ID | Unique, sequential, non-reusable identifier |
| Timestamp | When the decision was made |
| Initiator | Who or what made the decision (agent, department, kernel, human) |
| Decision Type | Creation, modification, deletion, approval, rejection, override |
| Target | What the decision affects |
| Rationale | Why the decision was made |
| Alternatives | Options considered (if applicable) |
| Evidence | Supporting data or references |
| Previous ID | If correction, the ID of the corrected record |
| Outcome | Result or status |
| `evidence_strength` | How well-supported this record is — see §12.2.A below |

#### 12.2.A Evidence strength on the decision log

> **Added — 2026-08-30.** Evidence: R12 (evidence-ledger append-policy), wave-2 comp-crm strong/weak/blank pricing, wave-3 Impeccable detector pattern. Pairs with §16.6.B.2 (per-task append-policy); this is the *column* that records which tier a given decision-log entry was written at.

The decision log schema (§16.1) gains an `evidence_strength` column with the three-tier enum from §16.6.B.2:

| `evidence_strength` | When written | Reader expectation |
| --- | --- | --- |
| `strong` | Auto-write on task completion with machine-verifiable backing | Treat as authoritative for §12.4 intelligence queries |
| `weak` | Owner/manager-settled from a worker suggestion | Treat as advisory until settled; settle by writing a correction (§12.3) referencing the original |
| `blank` | Auto-write when only completion evidence exists | Treat as fact-of-completion, not fact-of-quality |

The sanitization gate (§12.6 Layer 1) is unchanged. This column is orthogonal: it classifies the *quality* of evidence, not its *safety* to record.

### 12.3 Correction Mechanism

Every correction creates a new record with:

- A new Decision ID
- The same Timestamp as the original (preserving historical truth)
- A `Previous ID` field pointing to the corrected record
- A correction note as the Rationale

### 12.4 Decision Intelligence

The accumulated decision history becomes a source of organizational intelligence. The system can analyze patterns, learn from past decisions, and recommend approaches based on what worked before.

### 12.5 Organizational Archaeology

Because every decision is recorded with its context, the system can reconstruct the reasoning behind any state at any point in time. This allows full traceability of how the organization arrived at its current configuration.

### 12.6 Prevention, Correction, and Break-Glass Purge

**Memory is editable; history is not.** Department memory (§6) is a working store — it is revised, corrected, and overwritten as the organization learns, and needs no ceremony to change. The decision history (§12.1) is an audit trail. These are different kinds of object with different rules, and the rules below apply only to the second.

The design principle is **prevention over correction**: the decision log does not need a rich editing mechanism if content that should never be in it never enters it in the first place. Three layers, in order of how often they should fire.

**Layer 1 — Write-time sanitization gate (always on, no owner involvement).** Before any row is INSERTed into `decision_log`, its free-text fields (primarily Rationale and Evidence) pass a deterministic sanitization check for secret-shaped content — API keys, bearer tokens, private keys, connection strings, and similar credential patterns. Matches are stripped or the write is rejected outright, before the row exists. This is what makes "history should not contain that kind of content" an enforced property rather than an aspiration. The gate runs inside the Information department alongside the existing ACL/classification gate described in §16.6.

**Layer 2 — Correction (§12.3). The only routine mechanism for changing history.** Disagreement with, or an error in, a past decision produces a *new* record referencing the original via `Previous ID`. Nothing is masked, hidden, or removed. No special owner privilege is required. This covers effectively every legitimate need to change the record, and is fully specified in §12.3.

*Logical redaction was considered and rejected.* An owner-issued "hide this field from display" mechanism was specified in an earlier draft of this section. It is removed: its purpose was to suppress content that should not have been recorded, which Layer 1 now prevents at the source, and a display layer that hides parts of an audit trail undermines the trail's value for the sake of a problem that no longer exists.

**Layer 3 — Break-glass purge (owner-only, exceptional, explicitly confirmed).** Layer 1 is a pattern matcher, and pattern matchers have false negatives: a novel credential format, or genuinely sensitive content that simply is not secret-*shaped* (a third party's personal details, for instance), can slip through. Purge exists solely as the incident-response valve for that case — content that must not exist at rest at all. It is not a general-purpose editing tool and must never be presented as one.

Because this is the only mechanism that touches a row's physical existence, it follows this exact sequence, modeled on how destructive confirmations work in tools like GitHub's repository deletion:

1. Owner issues an explicit purge command naming the target Decision ID.
2. The system echoes back exactly what would be purged — the Decision ID, a preview of the flagged content, and why it is flagged — and states that confirmation requires typing that Decision ID back exactly. This forces the owner to engage with the specific record rather than repeat a memorized phrase.
3. Owner types the exact Decision ID back.
4. The biometric/key-based authentication already required for irreversible actions (§13.3) fires as the actual authorization gate. The typed-ID step is the human-readable confirmation; the biometric/key check is the cryptographic one — the two are independent, not substitutes for each other.
5. A permanent purge-record (what was targeted, when, why, by whom) is written to a small, separate, append-only companion table — *before* the purge executes. This companion table is never itself subject to purge: the fact that something was removed can never be erased, only the removed content itself.
6. The physical delete executes on the target row. The system confirms: "Deleted."

Every purge is, by definition, evidence that Layer 1 missed something. Each one should therefore also produce a sanitization-gate improvement, so the same class of content cannot reach the log again.

This preserves §7.2's promise of final owner authority without letting "immutable truth" quietly mean "immutable until the owner says otherwise." In normal operation the `decision_log` table is literally INSERT-only; purge is a rare, loud, self-documenting exception to that, not a routine capability.

### 12.7 Alternative Evaluation System

Before making significant decisions, the system automatically generates and compares alternatives. Each alternative is scored on criteria like cost, risk, alignment with goals, and expected outcomes. The chosen alternative and its competitors are all recorded in the decision history.

---

## 13. Security Model

### 13.1 Security Architecture

Security operates on three principles: **defense in depth**, **least privilege**, and **auditability**. Every access, every change, and every communication is logged and verifiable.

| Layer | Security Measure |
| --- | --- |
| Network | Local-only by default, encrypted tunnels for remote access |
| System | Sandboxed agent execution, resource limits |
| Information | Access control lists, permission-based read/write |
| Decision | Immutable history, override detection |
| Evolution | Sandboxed changes, rollback capability |
| Identity | Owner verification, secure credential storage |

### 13.2 Recovery Architecture

1. Failure detected
2. Identify scope and root cause
3. Isolate affected components
4. Restore from last valid state
5. Log incident, notify owner at next interaction

### 13.3 Security Policies

**Access Control Policy**

1. **Authentication**
   1. All agent-to-department calls require a valid session token
   2. Tokens expire after session or 24 hours
   3. Human owner uses biometric or key-based auth for irreversible actions
2. **Authorization**
   1. Departments define their own ACLs
   2. Cross-department access requires explicit permission
   3. Owner override bypasses all ACLs (logged)
3. **Audit**
   1. Every access attempt is logged
   2. Audit logs are append-only
   3. Owner can review audit logs at any time

### 13.4 Remote Access Policy

**Remote Access Policy**

> **Numbering correction — 2026-08-31.** These three groups were plain list items addressed in prose elsewhere as `§13.4.2` and its rules. The rest of this document uses `§X.Y.Z` for real sub-subsections, so `§13.4.2` read as an address that did not exist — eight passages cite it. Promoted to numbered sub-subsections here; **no wording is changed.** See `research/extraction/99-corrections.md` (finding C10).

#### 13.4.1 Connectivity

1. Remote access uses encrypted tunnels (tailscale, wireguard, or SSH)
2. Only the human owner initiates connections
3. All remote sessions are logged with IP, duration, and actions

#### 13.4.2 Restrictions

1. No automatic remote access for agents
2. External API calls go through a controlled gateway
3. All outbound traffic is logged and rate-limited

#### 13.4.3 Zero Trust

1. Every request is authenticated regardless of source
2. No implicit trust for internal traffic
3. Regular security audits and penetration testing

#### 13.4.4 Owner Companion Devices

**The owner's own devices are first-class remote clients — the phone included.**

> **Added 2026-08-31 at owner request.** §13.4 specified tunnels but never said what connects over them, and no passage in this document named a phone: every prior "phone" reference is a phone *number* in the §17.2 owner profile. §16.10.4's pairing-token ladder is device-agnostic — "a device joins by consuming a one-time pairing token" — so a phone fits the architecture that already exists. This subsection makes it explicit rather than leaving it to inference.

A companion device — phone, tablet, laptop — joins exactly as §16.10.4 specifies: a one-time pairing token consumed in-session, yielding a named device session bound to the owner's identity, individually revocable, with the revocation ledger append-only. It rides the §13.4.1 tunnel; it is not an exception to it. Three consequences follow from rules already in force, and are stated so they are not re-litigated per device:

- **Authentication is unchanged, not relaxed per form factor.** §13.3 item 1.3 requires biometric or key-based auth for irreversible actions. A phone is the *natural* home for Class A approvals (§16.10.2) because it is where the biometric hardware is — but it is the same requirement. A device that cannot satisfy §13.3 does not get Class A authority, whatever it is.
- **Render-don't-mutate still holds (§16.10.5).** A companion device renders and resolves gates; it executes nothing. Every approval remains a kernel-mediated owner decision recorded per §12, with Initiator: owner.
- **Sessions are logged as any other remote session** — IP, duration, actions (§13.4.1 item 3).

**OPEN ITEM — notification delivery to a disconnected phone. Unresolved, and flagged rather than decided, because the three options trade off different things and the choice is the owner's.** §16.10.2 parks an unresolved gate in the notification queue and resurfaces it on reconnect; §16.10.4 explicitly excludes "third-party cloud relay of console or gate traffic"; and §16.4 keeps Slack/Telegram out of V1 on the same privacy reasoning. A phone that surfaces gates only when opened is materially less useful, but the obvious remedy — APNs/FCM push — *is* a third-party relay, and a push payload carrying gate content relays precisely the traffic §16.10.4 forbids. The conflict is real, not a drafting gap:

1. **No push.** The phone pulls while connected; a disconnected gate resolves by expiry, deny-wins (§16.10.2). Fully consistent with §16.10.4 as written, at the cost that the owner learns of a parked gate only on open.
2. **Contentless push.** A wake-up signal via APNs/FCM carrying no gate content and nothing decryptable; the phone then fetches the payload over the tunnel. Preserves the prohibition on relaying gate traffic, but trusts a third party with *timing* metadata — that a gate existed, and when.
3. **Self-hosted delivery.** UnifiedPush with a self-hosted distributor, or a self-hosted ntfy instance — consistent with the self-hosted-only posture §16.4 already adopted for search. Requires an internet-reachable endpoint the owner controls, which sits against §13.4.1's "only the human owner initiates connections" unless that endpoint *is* the owner's own host.

Owner decision required before any of the three is built. Until one is chosen, the companion client is specified for the **connected** case — which needs none of them, and which is where the Class A approval value is.

---

## 14. Department Reference

### 14.1 AI Kernel Department (Executive Management)

- **Role:** Executive leadership of the entire organization
- **Authority:** All departments, resources, policies
- **Components:** Executive Mgmt, Strategic Planning, Internal Mgmt, Policy Enforcement, Resource Mgmt, Dept. Creation, Organizational Analysis, Cross-dept Coordination, Virtual Display/Computer Use, Self-Update
- **Self-Update as a kernel component, and the one self-modification exception in the system.**
  §16.6 forbids any worker from modifying LastHarness's own code or dispatch config. That invariant is
  unchanged. Updating LastHarness therefore cannot belong to any department, and is a kernel component
  by elimination as well as by design. The kernel **applies releases it did not author** —
  environment and system updates, and new LastHarness releases pulled from a versioned source. It
  does not generate or edit LastHarness source code; autonomous kernel code generation is explicitly
  out of scope. Every update follows this sequence, none of it optional:

  1. **Fetch and verify.** The release package is checksum-verified before it is treated as
     downloaded — the same integrity requirement §5.4 already applies to department packages,
     including the single automatic re-download retry on mismatch.
  2. **Sandbox.** The update is applied inside the Evolution Sandbox (§11.3): clone current state,
     apply in isolation, run the verification suite (§11.3.A) — Invariant always, Claim and
     Regression as the update requires. Tests fail → discard and log. This is
     §10's standing rule — "changes are never applied directly to production without going through
     this cycle" — and a kernel update is the last thing that should be exempt from it.
  3. **Owner authorisation.** A kernel update is irreversible in the sense §13.3 means it, and
     requires the biometric or key-based authentication §13.3 already mandates for
     irreversible actions.
  4. **Snapshot before promotion.** The current known-good kernel version is retained (§11.1
     Snapshot State) so §13.2's "restore from last valid state" has a state to restore to.
  5. **Promote and record.** A decision-log entry per §12 — Decision Type: modification, Initiator:
     kernel or owner, Target: kernel version, Rationale, Evidence (verification results), Outcome.

  **Rollback — the boot path restores, not the kernel.** §13.2 step 4 ("restore from last valid
  state") names the procedure; the open question was who executes it when the kernel itself will
  not start. The answer uses primitives this spec already has. Per §16.1, the Linux environment is
  launched as a genuine child process of LastHarness's own app, and per §17.2 the boot mechanism
  precedes and is independent of the kernel's runtime — the running boot process is by
  construction the pre-update code until the next successful boot, because an update is applied
  *inside* the environment it launches, not to the launcher. It therefore survives a kernel that
  fails after promotion. Recovery is §13.2 step 4 with its executor as the boot path: step 4 above
  retains the known-good version (§11.1) and keeps the boot path able to select it; a post-
  promotion boot failure, or a failed post-promotion verification, makes the boot path re-select
  the retained version and relaunch. This is A/B version selection on the mechanism that already
  boots the kernel — it extends §13.2's restore step by giving it an executor that is not the
  thing being updated; it creates no new component. The boot path selects only from versions the
  kernel itself snapshotted under steps 1–5; it does not bypass the sandbox, authorisation, or
  snapshot sequence.
- **Virtual Display/Computer Use as a kernel component, not a department:** the session lifecycle behind §16.7's `computer-use`/`browser-use` capabilities (provisioning, teardown, noVNC exposure) is shared infrastructure any department's role can request via Tool Profile — the same pattern as `llama-server` (§16.1), which is likewise kernel-adjacent rather than owned by one department. It is not a dedicated "Test/Preview" department because it isn't a business function of its own; it's a capability other departments' workers consume.
- **On-the-spot department creation:** the Dept. Creation component listed above is what executes acquisition path 2 of §5.3 — synthesizing a new department from an owner request, post-onboarding. Full workflow specified in §16.8.

### 14.2 Development Department

- **Role:** Software creation, maintenance, and infrastructure
- **Authority:** Codebase, build systems, deployment pipelines
- **Components:** Backend, Frontend, DevOps, Database, Quality Engineering
- **Role table.** §5.1's example config lists 3 roles (`backend_developer`, `frontend_developer`, `devops_engineer`); this section's Components line lists 5. The table below resolves the count in favor of 5 roles, matching Components; §5.1's 3-role config remains schema illustration only, not the literal roster.

| Role | Tool allow-list | Model alias | Memory scope |
|---|---|---|---|
| `backend_developer` | Read, Write, Edit, Bash, Glob, Grep | `coding` | `code_index` (read/write), `project_context` (read) |
| `frontend_developer` | Read, Write, Edit, Bash, Glob, Grep | `coding` | `code_index` (read/write), `project_context` (read) |
| `devops_engineer` | Read, Write, Edit, Bash (allow-listed: deploy pipelines, infra config), Glob, Grep | `tool-calling` | `build_history` (read/write), `project_context` (read) |
| `database_engineer` | Read, Write, Edit, Bash (allow-listed: schema/migration tools), Glob, Grep | `coding` | `code_index` (read/write, schema files), `project_context` (read) |
| `qa_engineer` | Read, Write (test files only, not app source), Bash (test execution), Glob, Grep | `coding` | `build_history` (write), `code_index` (read only) |
| `merge_coordinator` | Read, Write, Edit, Glob, Grep, Bash (allow-listed: git status/diff/log/branch, worktree add/list, merge --no-ff --no-commit — no push, no tag, no remote mutation) | `coding` | `code_index` (read), `build_history` (read), `release_context` (read/write) |
| `deployment_operator` | Read, Write (deployment manifests and staged release artifacts only), Glob, Grep, Bash (allow-listed: deployment plan/dry-run, artifact packaging, checksum generation) | `tool-calling` | `build_history` (read), `release_context` (read/write), `commit_history` (write) |

Development's `department.config.yaml` gains two memory categories alongside the existing ones: `release_context` (short-term) and `commit_history` (episodic).

**Addendum — 2026-08-11 (Development gains a Tier 1 invariant; supersedes the "no department-specific Tier 1 invariant" statement above).** The addition of `merge_coordinator` and `deployment_operator` gives this department an irreversible-action capability it did not previously hold, so the earlier statement that it carries no department-specific Tier 1 invariant no longer holds and is superseded.

**No-unauthorised-irreversible-publication invariant (Tier 1):** no role in this department executes an irreversible remote publication — push to a protected or default branch, tag creation, release publication, or production deploy — without the owner authorisation already required for irreversible actions generally (§13.3). The department stages the release; the owner authorises it. This mirrors the stage-don't-execute invariant §14.7 applies to `recovery_manager`, and applies here for the same reason: the department that identifies an irreversible action is not the one that should be able to take it unilaterally.

**Scope of the `merge_coordinator` no-commit rule.** The allow-list above permits `merge --no-ff --no-**commit**` for a reason: the merge into an integration branch is staged, not committed, so nothing reaches a shared branch without the owner. That rule governs the *integration merge*. It does not forbid — and was never intended to forbid — a worker committing accepted changesets inside its own dispatch worktree, which §16.6 requires as the producer of `success.commit_sha` (§16.6.B.1). The two are different operations on different refs: one is local to a throwaway worktree that dies with the dispatch, the other writes toward a branch other dispatches will read.

*Going-live assertion (§16.8 Tier 1):* in the Evolution Sandbox (§11.3), dispatch a `deployment_operator` task whose spec instructs it to push to the cloned repo's default branch. PASS = refused, terminal status `refused` (§16.6), no remote mutation observed. FAIL = any remote mutation.

*Manager precondition, not an invariant:* the department manager refuses to dispatch an integration task whose parent branch carries a failing build record. Deterministic check, Tier 2 for monitoring purposes.

**Addendum — 2026-08-12 (repo-map pre-step for Development dispatches — from the harness-research harvest).** Before dispatching a Development worker to an unfamiliar repo, the manager injects a **repo map**: a compact tree-sitter-derived skeleton of the repository (classes, functions, imports — the symbol graph, ranked by relevance) into the worker's context as part of its per-dispatch excerpt. Cost is ~1–2k tokens for global orientation that otherwise burns tens of thousands on `Glob`/`Grep` exploration and wrong-file edits. This is the Aider repomap technique applied to LastHarness's dispatch path, consistent with the `code_index` memory scope already in this department's role table (§14.2) and the CodeGraph/Repo Map wiring previously harvested (B7). Generated/maintained by the Information department's Automation jobs (§14.5), read at dispatch through the worker's allow-listed memory handle; no new access surface. Gated to subsections the worker's task legitimately touches, per §8.2's deny-by-default scoping.

### 14.3 Research Department

- **Role:** Discovery, analysis, and knowledge generation
- **Authority:** Research methods, sources, findings storage
- **Components:** Web Research, Data Analysis, Literature Review, Experiment Design

| Role | Tool allow-list | Model alias | Memory scope |
|---|---|---|---|
| `web_researcher` | WebSearch, WebFetch, Read, Write | `tool-calling` | `findings_corpus` (write), `research_history` (write) |
| `data_analyst` | Read, Write, Bash (analysis scripts, worktree-scoped), Glob, Grep | `thinking` | `findings_corpus` (read/write) |
| `literature_reviewer` | WebSearch, WebFetch, Read, Write | `smart` | `findings_corpus`, `research_history` |
| `experiment_designer` | Read (no web/bash — pure design reasoning) | `thinking` | `findings_corpus` (read), `research_methodologies` (write) |

No high-risk invariant is flagged for this department — its going-live check (§16.8 addendum) covers Tier 2 (live quality monitoring) only.

### 14.4 Intelligence Department

- **Role:** Inference management, model selection, cost optimization
- **Authority:** Model selection, inference parameters, cost decisions
- **Components:** Cost Analyzer, Quality Monitor, Cache Manager

| Role | Tool allow-list | Model alias | Memory scope |
|---|---|---|---|
| `cost_analyzer` | Read, Write + cross-dept read: resource-accounting field only (§8.2 standing exception) | `thinking` | `model_performance_history` (write) |
| `quality_monitor` | Read, Write + cross-dept read: transcript/trace field only, sampled (§8.2 standing exception) | `smart` | `past_recommendations` (write) |
| `cache_manager` | Read (llama-swap cache/stats endpoint), Write | `fast` | `model_performance_history` (write) |

**Recommendation-only invariant (Tier 1):** no role in this department writes changes directly into another department's config or memory — every model-selection or cost adjustment is written as a recommendation to this department's own memory and the decision log; the kernel or owner enacts it.

### 14.5 Information Department

- **Role:** Memory, knowledge, history, and governance
- **Authority:** All stored information, access rules, retention policies
- **Components:** Memory Dept, Document Store, Code Index, Decision History, Knowledge Base, Owner Profile, Security Vault, Session Archive, Skills
- **Owner Profile as an access boundary:** per §16.6, this component name is also the literal access-control category for memory-scoped dispatches, not just an organizational label — a dispatch with no legitimate need for owner-identity data (DOB, family/emergency contacts, religion, etc. — the §17.2 onboarding intake) has that category excluded from its memory allow-list by default, regardless of department or project.
- **Addendum — 2026-08-11 (Team Memory categories reconciled; no new department).** A review proposal named four "Team Memory" categories. Three are renames of categories this department already owns — Chat → Session Archive, Wiki → Knowledge Base, CodeGraph → Code Index — and are adopted by mapping, not by renaming: the §16.6 memory allow-list is category-keyed, and a rename invalidates every existing `memory_scope` entry that references the old name. One store, one name. The fourth, Skills, fills a genuine gap: §6.1's Procedural type had no named category in this department. It is added here as the `skills` category — the §6.1 Procedural-type store: skill registry YAML/config artifacts provisioned by §16.8 Step 2, referenced not copied, loaded by role at dispatch time (§6.3/§16.5), and nameable in a dispatch's memory allow-list like any other category.
  No Team Memory department is created. The proposed capability lies wholly within this department's existing §5.2 authority (all stored information, access rules, retention policies); a new department would both duplicate that authority and cross §5.4's boundary (discovery and distribution only, never authority). The `team-memory` registry entry is withdrawn, not published.

#### 14.5.A Skills Intake Lifecycle

> **Added — 2026-08-30.** Evidence: R5×8 in `research/extraction/11-department-blueprints.md` §1 (skill-pack supply chain the single most-recurred cluster across waves 1–5), strengthened by wave-2 oh-my-hermes 36-skill layer + qiaomu-seo strict-stale auto-block, wave-3 pin/quarantine-before-trust (agent-skills-flow vendir lock, gstack domain-skill quarantine, elevenlabs trigger-eval-before-register), and wave-5 provenance tagging convergence (codegraph `synthesizedBy`, loopx evidence-strength, ruflo signed witnesses). Pre-extraction gap was named twice before; this addendum closes it.

The `skills` category already exists in §14.5 and is provisioned via §16.8 Step 2. What it lacks is the *intake lifecycle* — provenance → scan → pin → register → re-validate — that the recurrence evidence demands. This subsection specifies that lifecycle.

**Provenance declaration.** Every skill registered in the `skills` category carries a `publisher` field identifying who authored it, mirroring Codebuff #147's typed `AgentDefinition.publisher` (wave-6 prior art). First-party and community skills alike declare their own publisher. Provenance is recorded in the registry, not in the skill body, so provenance can be updated without touching the artifact.

**Sandbox scan (one-time, blocking).** Before a skill enters the registry, it runs through a deterministic scan that examines the SKILL.md frontmatter, the script bodies, the references list, and the assets list. The scan enforces the §6.1 Procedural-type retention invariants (§16.8 Step 2 already names this), verifies that declared dependencies are present, and refuses skills that attempt to widen their own scope (the §16.6 invariant applied to skills themselves). A skill that fails the scan is not registered; the owner is notified per §14.6 with the failure reason.

**Version pinning.** Once registered, a skill is bound to a specific versioned artifact — a content hash plus the upstream publisher + version string when one exists (e.g. `npx skills add owner/repo --skill foo@1.2.3`). The skill is *referenced*, not copied, per §6.3; pinning happens in the registry record. A pinned skill is unchanged in the registry unless explicitly updated via the same lifecycle.

**Lazy loading.** A skill's content is not loaded into a worker's context by default. Per the `skills` category's role in §16.5's per-dispatch input gate, a skill is loaded *only when a dispatched worker explicitly invokes it* (via the role's tool allow-list or via a manager's pre-dispatch excerpt). This pairs with §7.1's `context_compaction_threshold` (see §7.1.B Compaction Protocol) to keep the working context small.

**Freshness governance.** A registered skill has a freshness window — a TTL on the *stale-by date* of any external source it cites (URLs, package versions). qiaomu-seo's dated-source registry (wave-2, #128) is the prior art: a skill citing an external URL that exceeds its freshness window is flagged for owner review via §14.6, not silently relied on. The window is per-skill, set by the publisher at registration; skills without a declared window default to the kernel's policy default (§7.1).

**Re-validation on update.** When a pinned skill's upstream version bumps, the registry does not auto-update; it stages the new version, re-runs the sandbox scan, and presents the diff to the owner (§16.10.2 Class B gate — registry update is side-effectful but reversible). On owner approval the new version is bound; on deny the old version remains.

**Re-validation on incident.** §14.7's incident rule DSL (§14.7.A) can fire a "stale or unsafe skill" rule; when triggered, the named skill is suspended pending re-validation. Suspension is a status change in the registry, not a deletion — re-validation restores it; a hard removal follows §16.8's department retirement procedure applied to a single skill.

*Sandbox assertion (§11.3):* clone the registry; (1) register a skill whose `publisher` field is missing; (2) register a skill whose scan reveals a credential-shaped string in a script body; (3) update a pinned skill to a new version without an owner gate. PASS = (1) refused at registration, (2) refused at scan, (3) refused at update with owner gate required. FAIL = any of the above auto-proceed.

| Role | Tool allow-list | Model alias | Memory scope |
|---|---|---|---|
| `knowledge_curator` | Read, Write | `smart` | Knowledge Base, Document Store — own dept only |
| `retention_auditor` | Read (cross-dept: metadata only — age/type/dept tags, never content), Write | `thinking` | `retention_rules` (write) |
| `owner_profile_steward` | Read, Write (Owner Profile category **only**) | `smart` | Owner Profile — exclusive |
| `session_archivist` | Read (session transcripts), Write | `fast` | Session Archive — own dept only |

None of these roles has any access to Security Vault, which stays kernel/Security-department infrastructure only. **Owner-initiated-only invariant (Tier 1):** `owner_profile_steward` may only be dispatched when the owner is the direct initiator — never triggered by another department's automated request.

### 14.6 Automation Department

- **Role:** Scheduled operations, background tasks, monitoring
- **Authority:** Scheduled jobs, cron tasks, periodic maintenance
- **Components:** Scheduler, Job Runner, Monitor, Alert System

| Role | Tool allow-list | Model alias | Memory scope |
|---|---|---|---|
| `scheduler` | Read, Write | `fast` | `job_definitions` |
| `job_runner` | Bash (allow-listed: worktree cleanup, log rotation, cache pruning — not arbitrary shell), Read | `tool-calling` | `run_history` (write) |
| `monitor` | Read, Write | `tool-calling` | `run_history` (write) |
| `alert_dispatcher` | Write (owner notification queue — narrow target), Read | `fast` | Owner notification queue |

**Kernel-routing-only invariant (Tier 1):** to trigger a task in another department, `job_runner` requests kernel routing — it never dispatches another department's worker directly, preserving the same central-policy-authority reasoning that excludes peer-to-peer coordination in §16.5.

### 14.7 Security Department

- **Role:** Safety, privacy, threat detection, compliance
- **Authority:** Security policies, access controls, audit enforcement
- **Components:** Access Control, Threat Detection, Audit Engine, Recovery Manager

| Role | Tool allow-list | Model alias | Memory scope |
|---|---|---|---|
| `access_control_reviewer` | Read (cross-dept: ACL config sections only, §8.2 standing exception), Write | `thinking` | `security_policy_history` |
| `threat_detector` | Read (cross-dept: audit logs, decision-log entries — security-relevant fields only), Write | `thinking` | `incident_log` (write) |
| `audit_reviewer` | Read (audit logs, decision log — read-only), Write | `smart` | `security_policy_history`, `incident_log` (read) |
| `recovery_manager` | Bash (allow-listed: isolate component, restore from snapshot — not general shell), Read, Write | `thinking` | `incident_log` (write) + incident-scoped cross-dept read |

**Stage-don't-execute invariant (Tier 1):** any destructive or irreversible action `recovery_manager` identifies routes through the owner-authorization gate already required for irreversible actions generally (§13.3) — it proposes and stages the fix, it does not autonomously execute it. This department's cross-department read access (audit logs, decision-log entries, ACL/config sections, every department) is a standing exception inherent to its §5.2 authority — see §8.2.

#### 14.7.A Incident-rule DSL

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

Rules are declared in YAML, compiled to SQL by the kernel at registry load, and **evaluated by this department's `threat_detector` role**, not by Automation's `monitor`. The reason is an access requirement, not a preference: the rules query `incident_log`, which is Security-department memory, and §14.6 gives `monitor` a memory scope of `run_history` (write) with no route to it. §8.2 permits cross-department visibility only as a declared standing exception in the requesting department's own config, and Automation declares none. `threat_detector` already holds the cross-department read exception covering audit logs and decision-log entries and already writes `incident_log`, so evaluation sits with the role that can actually see the data. `monitor` continues to cover Automation's own operational monitoring as before.

The DSL is deliberately narrow — match → aggregate → typed risk — and does not generalize to arbitrary computation; complex reasoning about an incident stays the `audit_reviewer` role's job.

Each rule carries an `evidence_required` list specifying the field set the rule must populate before the `then` clause fires. A rule that fires without its evidence set is a rule-engine failure, not an incident — the failure itself is logged.

**Detection window.** Rule evaluation is bounded by the retention of the tables it queries. `dispatch_event` is garbage-collected alongside `memory_working` (§6.3) — see §16.10.7 — so under the default retention window, incident detection has a **24-hour lookback**: a stale-skill reference or a cross-department ACL-violation attempt is only detectable while its event survives. Where a rule must detect over a longer horizon, the underlying event class needs retention longer than `memory_working`'s, set explicitly rather than assumed.

---

## 15. Agents: External Workers Integrated into Departments

Agents in LastHarness are not the system itself. They are **external workers** that departments hire, train, evaluate, and fire. An agent is a role filled by a model, not a fixed identity.

| Aspect | Traditional View | LastHarness View |
| --- | --- | --- |
| Agent purpose | The agent IS the system | The agent is a WORKER in the system |
| Model-agent relationship | One agent = one model | One role = one agent, model can be swapped |
| Agent permanence | Agent exists as long as its code exists | Agent exists as long as the department needs the role |
| Agent identity | Agent name IS the model name | Agent name IS the role name |
| Agent knowledge | Agent has its own memory | Agent uses department memory |
| Agent tools | Agent owns its tools | Department provides tools to agent |
| Agent evaluation | Manual, ad-hoc | Systematic, performance-based |

### 15.1 Agent Discovery and Onboarding

1. Discover agent tool
2. Test capabilities
3. Create role definition
4. Assign to department
5. Monitor performance

This is the onboarding-stage view of the full agent lifecycle in §9.1.

---

## 16. V1 Implementation Decision

> Status: **Locked** (core architecture decision). Narrower implementation-detail questions are tracked separately in §16.4.
>
> **Coding worker.** jcode is no longer part of the architecture. The Development department's coding tasks — and every other worker across every department — run as Claude Agent SDK subagents, differentiated only by role config (§16.5). There is no wrapped external CLI and no black-box tool worker category.

### 16.1 Stack

- **Kernel, Departments, Memory, Decision Log, Evolution Sandbox:** Python + SQLite + Anthropic Claude Agent SDK.
  - SQLite in WAL mode gives append-only immutable logging almost for free (INSERT-only `decision_log` table, corrections reference `parent_id`, never UPDATE/DELETE).
  - Claude Agent SDK provides the agent loop, context compaction, and subagent spawning out of the box.
  - **The kernel's own reasoning loop runs on local models, not Anthropic's cloud API.** Per §2.1/§17.2, the kernel handles the sensitive onboarding intake (DOB, religion, family/emergency contacts, etc.), which is why its locality is a standing property rather than a default. **No custom translation proxy is built or maintained by LastHarness.** `llama.cpp`'s server added native Anthropic Messages API support (`/v1/messages`, converting to OpenAI format internally before inference) in early 2026, and `llama-swap` passes the same endpoint through when fronting it. The kernel points `ANTHROPIC_BASE_URL` directly at `llama-swap`; the wire-format bridging is a first-party feature of software already in this stack, not bespoke LastHarness code. (This supersedes an earlier plan to build a thin custom Anthropic↔OpenAI proxy, drafted before this upstream capability existed.) Cloud Claude remains available as an explicit, opt-in path (paralleling OmniRouter's inclusion as a cloud-provider option in Appendix A Step 1), not the default.

- **Path B proxy exception (user-supplied endpoints only).** The no-proxy stance above covers LastHarness's own bundled local stack — `llama-server` already speaks native Anthropic Messages API, so no translation layer is needed there. It does not cover endpoints the user explicitly points LastHarness at (e.g. a self-hosted Ollama or LM Studio instance). For that case, and only that case, LastHarness includes a minimal one-way Anthropic-Messages→OpenAI-chat-completions adapter — a single small translation function, not general multi-provider routing infrastructure. If provider count or translation complexity ever grows past what one module can reasonably handle, that is a new decision to make explicitly, not a scope creep of this one.
- **Kernel locality is a standing property.** The kernel's own reasoning loop is always local, regardless of the inference provider selected during onboarding or afterward. The kernel handles sensitive onboarding intake (DOB, religion, family/emergency contacts, etc.), so locality is not a default that yields to a global setting — it is a standing property of the kernel specifically. Only department/agent inference follows the onboarding provider choice (cloud, custom/Path B, or local). The separate "Cloud Claude remains available as an explicit, opt-in path" toggle is unaffected — it was already independent of the onboarding default, and remains so.
  - **Addendum — 2026-08-11 (kernel-tier entities are not covered by this correction).** The locality guarantee above applies to the kernel's own reasoning loop specifically, not to every kernel-tier entity. The Personal Agent (§4.5) is kernel-tier and follows the onboarding provider choice like department/agent inference does, with the owner warned per Appendix A Step 1D. "Kernel-tier" classifies authority and ownership, not where tokens are computed. Project Agents (§4.6) follow the same rule.
- **Cloud workers are excluded — 2026-08-11.** "Every agent, worker, and dispatched tool runs inside it" refers to execution, not just inference. A cloud worker — a dispatched tool whose *execution* happens on a remote host controlled by a third-party provider — is not permitted. This is recorded, not newly decided: §2.1's "Self-hosted and private" and §13.4.2's "No automatic remote access for agents" already state it. A cloud *inference provider* (Appendix A Step 1 Path A) moves only the model endpoint and is a separate, already-specified surface. The Personal Agent's inference locality (§4.5) does not qualify as precedent: it moves inference for one kernel-tier entity, not execution for any dispatched tool.
- **Isolation model — one boundary, not two.** There is exactly one isolation boundary in LastHarness: **a Linux environment separated from the user's host OS.** Every agent, worker, and dispatched tool runs inside it, and nothing running inside it can reach the user's real Windows/macOS files or processes directly.
  - **Inference runs *outside* the boundary, natively on the host.** `llama-server` is a trusted first-party component LastHarness ships and controls — it is not an untrusted agent, and it executes no model-authored code. It therefore has no business inside a boundary built to contain untrusted agents. It runs natively on the host OS with native GPU acceleration (CUDA/DirectML on Windows, Metal on macOS), and agents inside the Linux environment reach it over a localhost socket. This is a **trust-boundary** decision, not a workaround for a technical limitation — see the note below.
  - **No per-task nested container.** An earlier draft specified spinning up a fresh throwaway container for every dispatch, *inside* the Linux environment. This is removed. It was a second layer of protection against a threat the first layer already covers, and it bought that redundancy at the cost of per-dispatch spin-up latency on exactly the modest consumer hardware this product targets. Workers run directly in the shared Linux environment.
  - **Git worktrees remain, and are not a security mechanism.** `git worktree add` gives zero-copy working-directory staging so concurrent tasks do not collide on the same checkout. That is a correctness and concurrency tool, not a containment tool, and it should never be described as isolation.
  - **Evolution Sandbox (§11.3)** clones state via git worktree and runs its verification suite inside the same Linux environment. Its safety property is *verify-before-promote*, not *contain-the-blast-radius* — the change is only applied to the live system after tests pass.
  - **Container runtime: no third-party container runtime at all, on either platform.** Governing principle, sharpened after two rounds of getting it partially wrong: the deciding property is not licensing, not daemon-vs-daemonless, and not vendor branding — it is whether the mechanism runs as a genuine subprocess of LastHarness's own frontend process, or exists as an independently-discoverable process a user could find and close. "Daemonless" was a red herring: a daemonless tool is still its own separately-named, separately-executable binary that shows up as its own row in Task Manager regardless of whether it keeps a background service running. This ruled out Docker Engine and Podman equally, on the same grounds — both are third-party binaries with their own process identity, whatever their internal architecture.

    - **Windows: WSL2 only**, launched as a genuine child process of LastHarness's own app — via the documented WSL API (`WslLaunch`/equivalent) or `wsl.exe` invoked directly by LastHarness, never through a container runtime layered on top.
      - Leading candidate, unconfirmed — verify hands-on: a sideloaded distro via `wsl --import`, reported not to register in Add/Remove Programs (unlike Microsoft Store-installed distros).
      - One irreducible exception: `VmmemWSL`, the system utility VM process, will appear in Task Manager regardless. This is acceptable — it's Microsoft-signed system infrastructure, closer to a Windows component than to bundled third-party software.
      - No shipped consumer product is known to bundle a Linux environment with the invisibility this requires; external research could not name one. Expect some irreducible visibility on Windows even at best effort.

    - **macOS: the `apple/containerization` Swift *package***, linked directly into LastHarness's own app — explicitly not the separate `apple/container` CLI tool.
      - Why the distinction matters, gotten wrong once already: `apple/container` is a standalone CLI built on top of the `containerization` package, and runs its own host-side background services (container-apiserver, image service, network service) to support that CLI — exactly the separately-discoverable-process failure mode this section exists to avoid.
      - The `containerization` package itself is different: designed for direct use by applications, using `Virtualization.framework` on Apple silicon with no extra binaries. This gets Apple's maintained kernel, OCI image handling, and the `vminitd` init system, without the standalone tool's separate services.
      - Open detail, not yet resolved: "linked directly into LastHarness's own app" assumes a Swift-native frontend (or at least a Swift binary in the same process). If LastHarness's frontend isn't Swift, `containerization` needs wrapping in a small compiled Swift helper, itself launched as a genuine child process — still satisfies the core requirement, but is unscoped implementation cost. Pin down LastHarness's frontend stack before treating this as closed.

    - **Linux: `systemd-nspawn` or `bubblewrap`, resolved.** The host already is Linux, so no VM is needed — but per-task isolation still needs real process/mount/network namespace separation, not just a bare `chroot` (filesystem-only, no process/network isolation).
      - `systemd-nspawn` ships with `systemd` itself (no separate daemon — it runs on invocation, not as a background service); `bubblewrap` (what Flatpak uses for sandboxing) is also daemonless. Both satisfy the same subprocess-of-our-own-app property as WSL2/`containerization` above.
      - Lower priority than the Windows/macOS cases, since the "looks like third-party adware" failure mode doesn't apply here — but no longer an open question.

  - **Docker Desktop, Podman, and Colima are all ruled out** — one decision, not three: all three are third-party binaries with independent process identity, which is disqualifying regardless of individual merits (Docker Desktop additionally brings a bundled first-party AI agent plugin and telemetry on by default; Podman and Colima are otherwise well-built tools that simply fail the subprocess requirement once GPU passthrough — their original justification here — stopped being needed).

  - **GPU passthrough is not a constraint on the runtime choice, deliberately.** An earlier draft made Apple Silicon Metal passthrough a hard requirement on whatever hosts the Linux environment. That requirement is dissolved by the decision above: inference no longer runs inside the boundary, so the guest never needs GPU access at all. Agent shells do not use the GPU; only `llama-server` does.
    - *Recorded so the constraint is not re-derived later:* GPU-accelerated inference inside a Linux container on Apple Silicon **is** technically possible today, via `libkrun`/`krunkit` exposing a virtio-gpu device with Mesa's Venus protocol forwarding Vulkan calls to MoltenVK on top of Metal. Red Hat's published benchmarks (developers.redhat.com, 2025) measured this approach reaching roughly 75–80% of native Metal throughput for llama.cpp inference in a containerized Linux VM — a substantial recovery from unaccelerated container performance. It was dropped because it is unnecessary here, not because it is impossible. Any future claim that GPU-accelerated containerized inference is impossible on macOS is false and should be checked against this note and its source.
    - Practical consequence: the runtime-selection field is wider, not narrower. No runtime is disqualified for lacking GPU passthrough.
- **Local inference stack:** `llama-server` (llama.cpp's HTTP server — OpenAI-compatible, and since early 2026 also natively Anthropic-Messages-API-compatible via `/v1/messages`) as the actual inference runtime, fronted by `llama-swap` to hot-swap which GGUF is resident in VRAM based on the `model` alias in the request; `llama-swap` passes through both protocols rather than only OpenAI's. Only one model is ever loaded at a time — required on consumer VRAM budgets, since a hardware-tiered pack can span up to 9 model categories that can't all be resident simultaneously. The kernel/departments call `llama-swap`'s endpoint directly using model aliases (e.g. `"coding"`, `"text-smart"`), never raw GGUF filenames, so the underlying model file can change without touching kernel code. For isolated testing/spikes where no swapping is needed, a caller can point at raw `llama-server` directly — `llama-swap`'s only job is the hot-swap itself, not protocol translation, which lives in `llama-server`.
  - **LiteLLM is deferred, not adopted, for V1.** It solves a multi-cloud-provider routing problem (10+ backends) LastHarness doesn't have at V1 — bundling it means shipping a Python runtime and 100+ package dependency tree in the Windows installer for a capability not yet needed. When cloud-provider support (OmniRouter, direct OpenAI/Anthropic APIs, etc.) is actually added, evaluate a thin custom routing proxy first and only pull in LiteLLM if provider count genuinely grows past what that can reasonably handle.
  - **Image-gen and video-gen are separate services**, each with their own endpoint (e.g. `stable-diffusion.cpp` for image-gen), not routed through `llama-server`/`llama-swap`. llama.cpp doesn't do diffusion, and the API shape (prompt → image/video file vs. prompt → text tokens) is fundamentally different. The kernel holds distinct client interfaces per modality rather than forcing everything through one text pipeline.
- **Installer (`installer/`):** an internal module inside the main LastHarness repo, not a separately branded/public project. Detects hardware, downloads the appropriate hardware-tiered model pack, and generates the `llama-swap` config mapping model aliases to GGUF paths. Treated as a hard dependency of LastHarness, not an optional plugin — held as an internal folder for now, with extraction into its own public project (tentatively "EasyLocalAI") deferred until/unless it proves reusable outside LastHarness.
  - The exact call mechanism between the kernel and `installer/` (in-process Python import vs. subprocess/local-endpoint) is not yet pinned down — resolve before implementation starts.

#### 16.1.A Single-artifact packaging contract

> **Added — 2026-08-30.** Evidence: R16 (single-artifact packaging contract incl. two-tier backup semantics), wave-2 pocketbase #130 one-binary shape. The existing `installer/` is "an internal module inside the main LastHarness repo" — this subsection specifies the installable shape.

A **LastHarness install bundle** is a single artifact per platform — one `.exe` for Windows, one `.dmg` for macOS, one `.AppImage` / `.deb` / `.rpm` for Linux, plus a single bundled `.iso` for air-gapped installs. The bundle carries:

1. The kernel + departments + manager dispatch runtime (Python + dependencies).
2. The local inference stack (`llama-server` + `llama-swap`, §16.1).
3. The selected hardware-tier model pack (§17.3) — **or** a deferred-download path for manual tier selection post-install. See *Model-pack delivery* below; the default is deferred.
4. The Linux environment seed image (WSL2 distro tarball for Windows; containerization image for macOS; systemd-nspawn/bubblewrap image for Linux).
5. A **two-tier backup metadata block**:
   - **Tier 1 (auto):** SQLite database + key Vault entries, written at the user's data directory. This is *the* backup that survives reinstalls of the app binary. **Scale-up track** — see §16.3 item 16. Cadence and snapshot consistency are specified below.
   - **Tier 2 (manual):** full install snapshot — model GGUFs, skills registry, kernel config — written to a user-chosen path on demand. Tier 2 is *not* automatic; auto-snapshotting tens of GB of model weights is rejected as wasteful. **Deferred to a §16.4 fast-follow** — see §16.3 item 16.

**Model-pack delivery — deferred is the default.** The two paths in item 3 are not alternatives of equal standing. §16.3 item 12 already specifies that the installer performs "hardware detection, model-pack download, `llama-swap` config generation," so **deferred download is the default**. The bundled model pack is what makes the air-gapped `.iso` an air-gapped artifact: a machine that cannot reach the network cannot defer a download, so an `.iso` that deferred its model pack would not be air-gapped — the one property that artifact exists to have. Stating the default this way also holds the build matrix at one artifact per platform rather than platforms × hardware tiers.

**Checksum coverage reaches the deferred payload.** The bundle is checksummed end-to-end (§5.4's department-package discipline extended to the installer). Two consecutive checksum failures surface a manual-retry UI; the user never sees a silent partial install. Because the model pack is deferred by default, the largest single payload in the install arrives *after* the bundle has been verified — so **the deferred download carries its own checksum verification on arrival and its own retry path**, discharging the same no-silent-partial-install guarantee. Without this, the guarantee covers the small half of the install and not the half most likely to fail mid-transfer.

**Tier 1 cadence and snapshot consistency.** Tier 1 is written on a debounce or interval cadence, not on every successful write: copying the database on every write is write amplification on the hot path, and the database is WAL-backed (§16.1), so a naive file copy of a live database can capture a torn state. The copy is taken via SQLite's backup API or `VACUUM INTO` so the artifact is internally consistent. This matters more than it appears — an inconsistent Tier 1 backup is worse than none, because it fails silently and is only discovered at restore.

This packaging contract is independent of the §16.1 isolation boundary — the boundary's *post-install* shape is unchanged. The contract specifies only the *delivery* shape: one file, verified, two backup tiers.

### 16.2 Why (Rejected Alternatives)

- **Rust from scratch:** rejected. Estimated 4–6 months for a solo developer to reach V1 vs. 3–5 weeks in Python; agent-loop/tool-calling/context-management plumbing would have to be hand-built.
- **Google ADK (as the orchestration framework):** evaluated as an alternative to Claude Agent SDK for the kernel/department layer and rejected as a full switch. Its typed agent hierarchy (`SequentialAgent`/`ParallelAgent`/`LoopAgent`) is more explicit to read than Claude Agent SDK's subagent spawning, but doesn't unlock dispatch capability the department-manager-to-worker pattern can't already express — the gap is ergonomic, not structural. Its built-in LiteLLM integration for local-model routing looked like a real advantage but isn't one: LiteLLM still ships either way, just pulled in by ADK instead of chosen directly — it relocates the dependency-weight problem rather than solving it. Against a solo-dev project with one local backend and a dispatch design already drafted against Claude Agent SDK, the switching cost (rewriting that dispatch layer, learning ADK's primitive semantics) outweighs the marginal benefit.
- **Unified Claude Agent SDK (chosen):** keep the organizational layer (kernel/policy/memory/log/sandbox) in Python on the Claude Agent SDK, and use that same SDK's subagent-spawning for every worker — coding included — rather than wrapping a separate external tool (see §16's coding-worker note for the jcode-removal decision this reflects). One runtime, one dispatch mechanism, one set of primitives (Skills, Plugins, MCP client, Subagents, and a portable computer-use tool — see §16.7) for every department, not a second Rust binary shelled out to for coding tasks alone. Coding workers get their capability from the role's tool allow-list (§16.5) rather than from a different underlying engine.

### 16.3 Scope and Build Order

**M0 — the spine.** The smallest thing that proves the dispatch contract (§16.6). One department, one dispatch path, no routing layer.

> **Milestones, not configurations.** M0/M1/M2 are *development milestones*, not shipped department sets. §4.3, §5.4, and §17.2 all state that Kernel, Memory (Information), and Security are always present and non-deselectable — that rule governs the active department set of a **shipped installation**, not the order in which the system is built. An M0 build is an deliberately incomplete system, not a valid configuration, and nothing here grants an exception to the "always present" rule for shipped builds.

1. **SQLite WAL schema** — the `decision_log` table (INSERT-only; corrections reference `parent_id`, never UPDATE/DELETE per §16.1) plus the minimum dispatch-record table.
2. **Decision-log write path** — append and read, with the immutability guarantee enforced at the storage layer, not by convention. The §12.6 Layer 1 sanitization gate is part of this path, and §12.6 specifies that it "runs inside the Information department" — a department that does not exist until M1. **At M0 the gate runs as a direct kernel call and migrates into Information at M1.** Stated rather than assumed, so the divergence is visible and has a named endpoint instead of becoming a silent inconsistency.
3. **One dispatch, end to end** — task spec in → fresh git worktree (§16.6 Input 2) → Claude Agent SDK subagent under a deny-by-default tool allow-list → capture the artifact diff excluding `.lastHarness/` (§16.6 Output 2) → the manager writes the decision-log entry, post-completion, never the worker.
4. **Tests that earn the claims** — no UPDATE or DELETE against `decision_log` is reachable through any code path; the allow-list actually denies; the manager reads git rather than worker self-report.

Deliberately *not* in M0: the kernel routing layer, the department registry, YAML config loaders (one role config lives inline for now), the MCP memory handle, result-schema validation, the policy engine, and any second department.

**M1 — make it a kernel.** Prove that a second department is configuration, not surgery. This is the rework guard: if adding a department here costs more than a config file plus a role table, the dispatch contract is wrong and M0 has just paid for itself by revealing it early.

5. Kernel — department registry (with the §16.8 lifecycle status field), routing, policy enforcement, logged (§4.4).
6. Config loaders — `department.config.yaml`, `manager.config.yaml`, `policies.yaml` (§5.1, §7.1).
7. Department-scoped memory, persistent across sessions — the MCP memory handle and its deterministic Information-department gate (§16.6, §14.5).
8. Result schemas — success and failure (§16.6 addendum 2026-08-12, §16.6.B).
9. **Information (§14.5)** as the second department, walking the registry lifecycle. It is chosen rather than arbitrary: §4.3/§5.4/§17.2 make it non-deselectable, it is where the §12.6 Layer 1 sanitization gate belongs (which M0 runs in-kernel), and it owns the department-scoped memory in item 7. Development remains the dispatch-contract reference implementation; Information proves the registry is genuinely config-driven and takes the gate home.

**M2 and beyond — scale-up track, roughly in this order.**

10. The remaining four departments — Research, Intelligence, Automation, and Security — each registered and gated per §16.8 before its status moves to `active`. Development (M0) + Information (M1) + these four = six.

11. **Evolution sandbox** — propose change → clone state (git worktree) → verify → promote or discard, all recorded immutably (§11.3). Its verification suite is defined in §11.3.A: three layers — Invariant (always), Claim (the change's own assertion), Regression (everything already asserted) — the consumer selects which run, and an empty layer fails closed. §16.8 Step 4 routes every department's going-live validation through this same mechanism.

12. **Installer (`installer/`)** — hardware detection, model-pack download, `llama-swap` config generation. During M0–M2, point the stack at an existing **local** `llama-swap` instance rather than building the installer first — packaging ahead of a finished machine is plumbing for a shape that will change. A cloud endpoint is acceptable during development for *department and agent* inference, but **never for the kernel's own reasoning loop**, which §16.1 makes local unconditionally. Routing the kernel through a cloud endpoint would be a §16.1 correction requiring the explicit treatment §16.8 reserves for that class of change — not a development convenience.

13. **Hardware-tiered model packs** — 9 categories: smart, mid, fast (general text), coding, image-gen, video-gen, vision, thinking, tool-calling. **7 wired** (smart/mid/fast text, coding, vision, thinking, tool-calling); **image-gen and video-gen deferred** (see §16.4). All 9 remain visible in the pack-selection UI — the deferred two are marked "coming soon" rather than hidden. Each category has device-appropriate presets selected at install time (see §17.3).
14. **Onboarding questionnaire** — personal-memory intake feeding the Memory department (see §17.2).
15. **Owner surfaces, subset (§16.10)** — the console's read-only render (§16.10.1), the worker-event stream (§16.10.7), and Class A gates (§16.10.2, already §13.3 mechanics). Class B gates (§16.10.2) and owner-editable plan artifacts (§16.10.3) remain deferred: both are specified and adopted, but each carries real dispatch-path work.
16. **Install bundle packaging (§16.1.A), subset** — one artifact per platform, **deferred model-pack download as the default** (the bundled pack ships only in the air-gapped `.iso`, since a device with no network cannot defer a download). **Tier 1 backup** (SQLite + key Vault entries, on a debounce cadence, snapshot-consistent via `VACUUM INTO`, verified at write time). Tier 2 backup is manifest-only; weights are excluded by design.
17. **Skills intake lifecycle (§14.5.A)** and **incident-rule DSL (§14.7.A)** — §14.5.A's provenance → scan → pin → register → re-validate lifecycle over a `skills` table; §14.7.A's owner-authored `when` DSL compiled to SQL over `dispatch_event` (and `skills` via subquery), evaluated into an `incident` table, with `suspend_skill` closing the loop back into the skills registry. Surfaces are CLI/TUI-first; the GUI console (§17.4) is item 18.

18. **GUI owner console (§17.4)** — §17.4 states LastHarness is "GUI-first" targeting non-technical §17.1 users, yet items 1–17 specify only CLI + TUI surfaces. The graphical console is its own milestone, off the critical path; items 14–17 are specified CLI/TUI-first.

**On the six-department commitment.** The six departments remain the intended shape of the system; nothing above reduces the target. What changed is the *order*: one department proves the contract, two prove the registry, six arrive through the §16.8 gate rather than all at once. Role definitions, tool allow-lists, model aliases, and memory scope for each department live in that department's own `department.config.yaml` (§5.1 schema), not in this document — mirroring how Development's own config was never inlined here either. Every newly-wired department is subject to the going-live validation gate specified in §16.8 before its status can move to `active`.

**Role detail location.** Full role tables for all six departments, Development included, are inlined in §14 as the authoritative reference. `department.config.yaml` remains the artifact the running system actually reads, but is not the sole location of this detail — the two are kept in sync manually until/unless that becomes a real maintenance burden, at which point generating one from the other is worth revisiting.

### 16.4 Implementation Questions — Resolution Log

Narrower, independently-resolvable questions raised during V1 design, tracked separately from §16's "Locked" core-architecture status above. Five of six are closed; one has two verification items still open.

- ~~**All 9 model categories in V1**~~ — 7 categories (smart/mid/fast text, coding, vision, thinking, tool-calling) wired for V1 launch; image-gen and video-gen deferred to a fast-follow, since they require separate serving stacks (not `llama-server`/`llama-swap`) with much heavier download/runtime cost — the scope risk was really concentrated in those two, not the other 7, which share one pipeline and cost little marginally. All 9 stay visible in the pack-selection UI regardless, per §16.3.
- ~~**`installer/` call mechanism**~~ — In-process Python import. `installer/` is plain Python living in the kernel's own repo, so there is no language or process boundary to justify a subprocess/local-endpoint pattern. It is imported directly.
- **Container runtime for the Linux environment — resolved, 2 verification items open.** Per §16.1: WSL2 only on Windows (no Docker/Podman on top), the `apple/containerization` Swift package linked directly into LastHarness's app on macOS (not the separate `apple/container` CLI, which carries its own background services), and `systemd-nspawn`/`bubblewrap` on Linux (no VM needed, but real namespace isolation rather than bare `chroot`). Still open: (1) hands-on confirmation that `wsl --import` sideloaded distros actually don't register in Add/Remove Programs, and (2) scoping how `containerization` gets embedded if LastHarness's frontend isn't itself Swift-native — this may require a small compiled Swift helper process, a real implementation cost not yet sized.
- ~~**Coding worker architecture**~~ — Every worker, coding included, is a Claude Agent SDK subagent (§16.5/§16.6). No wrapped external CLI, no fork.
- ~~**SaaS search providers**~~ — resolved 2026-08-11: **self-hosted only for V1** (SearXNG; no Exa, Tavily, or cloud Firecrawl). No account, no key, no third-party query log. A SaaS option (off by default, §13.4.2 gateway + scrub + allow-list, persistent Appendix-A-Step-1D-style warning) stays unclosed but unadopted — revisit only after real quality data from SearXNG in the field.
- ~~**Messenger surfaces (Slack/Telegram)**~~ — resolved 2026-08-11: **out of V1.** §14.6's `alert_dispatcher` already writes to the owner notification queue with a narrow target; extending that queue to a third-party messenger is a distinct privacy decision and is not in the §16.3 build order. If later taken, it reuses the persistent-warning pattern.

### 16.5 Department Manager → Worker Dispatch

A department manager is itself a full agent — its own instructions, tool whitelist, skills, and MCP connections. Workers do not inherit any of this by default. Per §6.2 and §15, tools and memory belong to the department, not to any individual agent — and that includes the manager. Dispatch to a worker is scoped per-role, not inherited from the manager's own config.

**One worker category.** Every worker — coding, judgment, dialogue, browser-driving, desktop-driving — is a Claude Agent SDK subagent. There is no separate black-box tool-worker path; that category existed only because jcode was an external, non-SDK tool, and jcode is no longer part of the architecture. Each role (e.g. `backend_developer`, `frontend_developer` per the §14.2 role table) is a full agent persona: its instructions, tool whitelist, skill list, and MCP connections are defined once — §14's role tables are the authoritative reference, read at runtime as a config slice in `department.config.yaml` — independent of the manager's own config and independent of every other role's.

Dispatching to a role spawns a fresh subagent session via the Claude Agent SDK's native subagent-spawning, scoped to exactly that role's config slice plus a bounded, task-relevant excerpt of department memory (per the token-budget limits in §4.2). The subagent session is ephemeral — it exists for the task, then terminates; its output, not its process, is what persists. See §16.6 for the full dispatch contract every worker follows regardless of role.

What differs between roles is entirely the config slice, not the mechanism: a coding role's tool allow-list includes Read/Write/Edit/Bash/Glob/Grep; a role that needs to drive a web page includes the `browser-use` MCP tool (§16.7); a role that needs a full visual desktop includes `computer-use` (§16.7); a purely conversational role includes neither. Gating capability by role config, not by worker type, is what makes jcode's removal a simplification rather than a loss of capability — every capability jcode's dispatch path used to carry (shell/file access, code editing) is now just another entry in a role's tool allow-list, subject to the same Tool Profile scoping as everything else in §8.2.

**Internal subagents are not the same thing as peer coordination.** A worker may spawn its own scoped child subagents in the course of one dispatch (the SDK supports this natively) without that counting as the kind of cross-task orchestration reserved for the kernel. The distinguishing property: a spawned child is bounded to its parent's task, budget, and lifetime, and terminates when the parent dispatch ends — it never outlives or escapes the dispatch that created it, and it never coordinates with workers outside that dispatch. That's what jcode's `swarm` mode was excluded for (§16.2) — peer-to-peer coordination with no central policy authority, workers deciding among themselves rather than being dispatched by a manager. Scoped, task-bounded child subagents don't have that problem and are not restricted by this rule.

**Addendum — 2026-08-12 (Peer-to-peer agent messaging: flagged conflict, not adopted).** Prime Agent's README advertises "agents communicate directly" — running agents exchange messages and orchestrate one another without routing through the user. That is peer-to-peer coordination with no central policy authority, which is precisely the capability §16.2 excluded when jcode's `swarm` mode was removed, and §4.4 reserves to the kernel. It is therefore **flagged as a conflict, not adopted**: LastHarness routes all inter-agent communication through the kernel (§4.4), and the kernel-accountable manager (§16.9) is the only dispatch authority. Same category, same disposition: Prime's **daemon-backed background sessions and reattachable agents** would be standing processes, violating §16.5's ephemeral-dispatch invariant — the compatible path for "keep long work moving" is the Automation-department scheduled re-check (§14.6) against stored goal state, never a resident worker. Adopting either would require the explicit-correction procedure the §16.8 OPEN ITEM mandates; neither is a V1 candidate.

From the manager's perspective, dispatch is a single action in its toolset regardless of role. Worker output is written back to department memory and the immutable decision log (§12) — never to any memory owned by the worker itself, since per §6 workers hold no persistent memory of their own.

### 16.6 Worker Dispatch Contract

Every worker dispatch — regardless of department, role, or which tools that role's config grants — follows the same contract. This section defines what the kernel/manager provides to a worker and what a worker must return, independent of task content.

#### 16.6.A Kernel-composed handoff packet

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

**The kernel composes; the worker spawns.** The parent submits the handoff intent (§4.4 cross-department request shape, simplified for in-dispatch) and the kernel composes the packet from the parent's role config, the child's role config, and the dispatch's accumulated state. **Spawning remains worker-native** within the §16.5 bounds — the kernel does not spawn the child. This is the §4.4 rule applied to in-dispatch delegation: kernel-mediated only, never peer-to-peer. Packet composition is a new kernel responsibility; dispatch mechanics are unchanged.

**Leases are intra-dispatch-tree only.** `leases.parent_worktree_path` grants a child read access to its parent's worktree — a sensible alternative to copying state, and permitted *within one dispatch tree*. It is never a mechanism by which two concurrent dispatches come to share a worktree: §16.6 Input 2 gives each dispatch a fresh worktree precisely to give "working-directory separation between concurrent tasks," and the lease does not override that.

The packet is logged as a `dispatch_event` (§16.10.7) with class `dispatch.handoff`. The §16.9 manager's `previousRun` argument (Codebuff #147 SDK reference, waves 5–6) is a degenerate case of this packet without the leases/claims/continuation fields.

**Inputs, provided at dispatch time:**

1. **Task spec and acceptance criteria** — from the dispatching manager.
2. **Working directory** — a fresh git worktree (§16.1), giving working-directory separation between concurrent tasks. Not a security mechanism on its own; containment against the host is the Linux environment's job (§16.1).
3. **Model endpoint and alias** — the role's subagent is pointed at `llama-swap` via a model alias (e.g. `"coding"`, `"tool-calling"`), never a raw GGUF path, per §16.1.
4. **Tool allow-list, deny-by-default** — the role's config slice from `department.config.yaml` (§16.5), mirroring §8.2's per-role tool whitelist. A role's config determines everything it can do; there is no parameter the worker can set to widen its own access.
5. **MCP memory handle, structurally scoped at dispatch time** — bound to one (department, project) pair and an explicit allow-list of memory *categories* (mirroring §8.2, applied to memory categories — e.g. Code Index and project-scoped episodic memory for a typical Development task, never Owner Profile, Security Vault, or another project's memory; see §14.5). This is the worker's only memory access for the duration of the task; there is no separate worker-side memory store. Deny-by-default here too — no project field the worker supplies itself.
6. **Per-dispatch instructions** — role-specific system-prompt content injected at session start, scoped to that role, not inherited from the manager's own instructions (§16.5).
7. **Token ceiling and wall-clock timeout** — per §4.2's resource limits.

**Outputs, returned to the manager:**

1. **Machine-readable terminal status** — success / failure / timeout / refused. Read from the SDK session result, not scraped from prose.
2. **Artifact diff** — the worktree's `git diff` is the canonical record of what changed. This is deliberately backend-agnostic: the manager reads git, it doesn't trust the worker's self-report of what it did. The diff is computed **excluding `.lastHarness/`**, which holds dispatch-internal state such as the compaction manifest (§7.1.B); that state exists to describe the dispatch, not to be part of its recorded output.
3. **Dispatch trace** — a span-per-call record of the session: one span per tool invocation carrying `tool`, `call_seq`, `args_digest`, `result_bytes`, `tokens_in`, `tokens_out`, `latency_ms`, and `status`. Adopt #64 LangFuse's span-per-call shape (`research/extraction/09-feature-extraction.md`); before this the output was named but never given a schema, so two implementers would have produced two incompatible traces. The trace feeds the Rationale and Evidence fields of the decision-log entry below.

   `args_digest` rather than raw arguments is deliberate. The trace lands in the decision log's evidence field, and per §12.6 a record that leaks a credential while claiming to be an audit trail is worse than no record at all — the digest preserves "was this the same call as that one" without persisting secrets.

   The dispatch trace is the *completed record* of a session. It is not the same artifact as §16.10.7's worker-event stream, which is a liveness substrate for watching a dispatch while it runs; the trace is written once at session end.
4. **Token/resource accounting** — for §4.2 enforcement.

**Commits during the session.** A worker commits each accepted changeset to its own worktree as it goes — one commit per coherent unit of accepted work, never one commit at the end, and never a commit of scratch state. This is adopt #91 Aider's commit-per-changeset discipline, and it does three things at once: it gives §16.6.B.1's `commit_sha` a producer, it anchors the diff below to a sequence of reviewable states, and it yields rollback checkpoints without a separate snapshot mechanism.

Commits here are **local to the dispatch worktree and never pushed.** The dispatch's output is still the worktree's `git diff` (Output 2); the commit history is what makes that diff legible rather than opaque. §14.2's `merge_coordinator` rule — `merge --no-ff --no-**commit**`, no push, no tag, no remote mutation — governs the *integration merge into the parent branch*, not commits made inside a dispatch worktree. The two do not conflict: the worker commits within its worktree, while the merge into the integration branch is staged rather than committed and the owner authorises publication under §13.3.

Memory writes are not part of the return value — they already happened live over the MCP memory handle during the session (see below).

**Memory writes during the session.** Writes through the MCP memory handle pass a **deterministic gate inside the Memory department — the memory component of the Information department (§14.5)** — ACL/category check, memory-type classification (§6.1), immutable logging (§12) — not a manager-agent reasoning call for every write. The gate escalates to the manager/kernel only for genuinely ambiguous cases: a contradiction with existing memory (§12.3's correction mechanism — a new record referencing `parent_id`, never a silent overwrite), an unclear memory-type classification, or what looks like an unregistered project. The worker never resolves these itself — deciding whether a new project should exist is always a kernel-level decision (§4.1), never a worker's unilateral call.

**Escalation resolution: scratch-scope, not pause.** When the gate escalates, the worker is immediately handed a temporary, unattributed scratch scope so the task keeps moving rather than blocking on the ambiguity. In parallel, the owner is alerted — immediately if reachable, or queued and delivered at the owner's next interaction if not, mirroring the notify-on-return pattern already used for incident recovery in §13.2. Once the kernel/manager resolves the ambiguity, scratch-scoped writes are relabeled into the correct project/category or discarded if redundant.

**On failure or timeout, the worktree is preserved, not discarded**, so the owner can inspect partial work — the disk cost of keeping it is small next to the debugging value of a non-empty diff from a failed run. It's cleaned up on the next successful dispatch to the same task, or by the Automation department's routine maintenance if never revisited. (Default assumption, not extensively discussed — flag if a different disposition is wanted.)

**Decision-log entry.** A task's final outcome is recorded separately from its incremental memory writes. The live MCP writes during a session capture facts learned along the way (e.g. "the login endpoint lives in `routes/auth.py`"). Task completion additionally produces one proper decision-log entry (§12) — Decision Type, Target, Rationale, Outcome, and a Previous ID if it's a correction — written by the dispatching manager once the worker session concludes, never by the worker itself. This is what lets the Decision History answer "why did this task happen and what came of it" at a glance, rather than leaving only a trail of scattered facts.

**Invariants, independent of role:**

- No durable state outside the worktree and MCP-mediated memory — no worker-native config or memory directory is provisioned.
- No self-modification — a worker never modifies LastHarness's own code or its own dispatch config.
- A worker never writes to `decision_log` directly; only the dispatching manager does, post-completion.
- A worker cannot widen its own scope — every allow-list above is set at dispatch time by the manager/kernel, not requested by the worker.

**Addendum — 2026-08-12 (strict-anchor edit contract; per-role result schemas — from the harness-research harvest).** Two dispatch-contract hardenings, decided after reviewing edit-discipline and subagent-return evidence:

1. **Strict-anchor edit contract (B1 hardening).** Edits are `(anchor, replacement)` operations where the anchor is a content-derived hash of the target line/block in the worktree's current state. The manager/kernel applies the edit by locating the anchor byte-exactly; on a hash mismatch the edit is **rejected with a loud terminal error, never fuzzily applied**. The model re-reads and re-anchors. Rationale: retype drift is the dominant coding failure mode, so converting "edit" into a machine-checkable op and deleting the silent-corruption failure class is a correctness lever, not a style preference. Consistent with §16.6's regnant principle — the manager reads git, it doesn't trust self-report — now applied at the edit granularity, not just at completion. Research basis: agent-computer-interface design moves fixed-model success (SWE-agent, arXiv:2405.15793); anchor-verified editing is the same principle applied per-edit. Recorded in the spirit of B1 as previously harvested; this formalizes it into the contract.

2. **Per-role result schemas.** The dispatch contract's Output item 1 (machine-readable terminal status) is extended to a full **result schema per role**: the manager validates the worker's returned artifact type/shape (e.g. the schema field for `backend_developer` returns) against the role's schema in `department.config.yaml`; a non-conforming return is rejected as `refused` with a reason, never parsed as prose. The parent context only grows by verified signal — the parent contracts no raw transcript ingestion. Research basis: subagents as context firewalls (Anthropic multi-agent engineering posts, 2025); context degrades with length and noise (Lost in the Middle, arXiv:2307.03172), so schema-gated returns are a context-quality control, not bureaucracy. No new tool surface — the schema lives in the existing role config slice read at dispatch time (§16.5).

#### 16.6.B Result Schemas v2

> **Added — 2026-08-30.** Evidence: R6×3 (structured partial-failure channels & typed errors) + R10×4 + R12 (evidence-ledger append-policy), wave-2 comp-crm write/suggest/prefer-blank thresholds, wave-3 Impeccable #132 deterministic detector pattern as production prior art. The existing 2026-08-12 addendum specifies per-role *success* return shapes; this addendum specifies per-role *failure* return shapes and the evidence-ledger policy that decides what gets written to the decision log.

**§16.6.B.1 Typed failure channels.** Per the existing 2026-08-12 addendum, every role's `department.config.yaml` carries a `result_schema` field for success returns. This addendum extends the same config slice with a `failure_schema` field, also a typed object specifying the shape of `failed` / `refused` / `timeout` returns. The schema declares which fields a manager can rely on for *each* terminal status:

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

`success.commit_sha` is populated by the commit-per-changeset discipline above (§16.6 Outputs): the worker commits accepted changesets to its dispatch worktree during the session, so the head commit of a successful dispatch identifies its final state. A role whose work produces no worktree change — a pure research or analysis task — returns `commit_sha: null`, and that is a legal success, not a schema violation.

**§16.6.B.2 Evidence-ledger append-policy.** Every dispatched task eventually produces §12 records. The decision log is *append-only* and *immutable* (§12.1), but the **strength** of what is written is not uniform. Wave-2 comp-crm's evidence-ledger pricing is the prior art: writes are priced by the *confidence* of the underlying evidence, not by whether a worker *wants* to write.

This subsection specifies a three-tier append-policy:

| Confidence | Append behavior | Source |
| --- | --- | --- |
| **Strong** | Auto-write to decision log on task completion | Test results, build outputs, LSP diagnostics, validator reproductions, decision-log entries referenced by the dispatch |
| **Weak** | Surface as a *suggestion* in the owner notification queue (§14.6), pending owner or manager settlement before becoming a decision record | LLM-inferred outcomes, free-text rationale, prose self-reports, unverified worker claims |
| **Blank** | Write a minimal completion entry only — **no quality claim**. Prefer empty evidence over confidently-wrong evidence — a "blank beats confidently-wrong" rule | Worker self-assessments without machine-verifiable backing |

The kernel-side gate that enforces this lives in the Information department's memory handle (§16.6, §14.5). The gate is deterministic, not a model call. Workers that emit only blank-evidence outputs produce minimal decision-log entries (`Decision Type: blank-completion, Outcome: completed, Evidence: none`) — the log records the *fact* of completion, not an unsupported claim about quality.

**"Blank" means no claim, not no record.** The Blank tier suppresses the *quality assertion*; it does not suppress the row. A blank-evidence dispatch still produces its completion entry, marked `evidence_strength: blank` per §12.2.A, because a dispatch that happened and left no trace is a worse failure than one recorded without a verdict. §12.2.A's "auto-write when only completion evidence exists" and this subsection's "do not write" mean the same thing at different granularity: do not write *an unsupported claim*, do write *the completion*.

This policy pairs with §16.6.B.1: a `failure_schema.reason_code` field is itself strong evidence (the worker's own typed refusal); a `failure_schema.explanation` free-text field is weak evidence.

**Grading changes the entry, not its author.** Evidence tier determines an entry's *content* and its `evidence_strength` value (§12.2.A) — never *who writes it*. In all three tiers the entry is written by the dispatching manager once the worker session concludes, never by the worker itself, exactly as the §16.6 decision-log rule requires. Owner settlement of a weak suggestion changes what the record says; it does not make the owner or the worker the writer of record.

**Addendum — 2026-08-12 (RLM prompt-as-a-variable: design note, not a build).** Prime Agent's core abstraction treats context as named, reassignable variables and tools/subagents as function calls inside a persistent REPL. This is a distinct context-engineering technique (context-rot and length-degradation literature, F4 family): linear append-only transcripts degrade with length, while named reusable slots can be overwritten in place to hold the context to a stable size. It is **recorded as a design note, not adopted as a mechanism**: LastHarness's context discipline is per-dispatch (bounded excerpt + compaction at `context_compaction_threshold`, §7.1), and the RLM's resident-REPL form conflicts with §16.5's ephemeral dispatch. The *principle* worth carrying into the contract — keep the working context as a small set of deliberately-managed slots rather than a growing transcript — is already what the §7.1 compaction threshold and §16.6 memory-excerpt bounds express; no new mechanism required at V1.

#### 16.6.C Deterministic enforcement hooks

> **Added — 2026-08-31.** Evidence: adopts **#2 ECC, #29 Deepseek Harness, #44 oh-my-openagent** — three tools converging independently on the same substrate — recorded as meta-pattern §E.1 in `research/extraction/09-feature-extraction.md`. Harvested by `research/extraction/14-research-rescan-2026-08-31.md` §C (M1 and M4). Before this pass the word "hook" did not appear anywhere in the specification.

This contract specifies a number of gates: the deny-by-default tool allow-list (Input 4), the deterministic memory-write gate, the §8.2 ACL layers, the §12.6 sanitization gate, the per-child budget in §16.6.A, the compaction trigger in §7.1.B. Until now it has never named the mechanism that enforces them. That mechanism is the **lifecycle hook**: a deterministic interceptor the kernel installs around every tool invocation in a dispatched session.

**Everything deterministic is a hook; nothing deterministic is a request.** A gate that can be decided without judgment — is this tool on the allow-list, does this memory write exceed its category, has this worker made the same call four times running — runs as code at the hook boundary. It is not asked of the model and it is not something a worker can decline. This is what makes §16.6's deny-by-default a *code-enforced* property rather than an instruction a worker is trusted to follow.

Hooks run in-process inside the dispatch. They are not a standing process, not a sidecar, and not a separate service — they are function calls the kernel wraps around the SDK's tool-call lifecycle, and they die with the session. §16.5's ephemeral-dispatch invariant is untouched: the hook set is constructed at dispatch time and discarded at session end.

**Mandatory hook set.** Every dispatch carries at least these; a department may add more in its `department.config.yaml` slice, and a role may narrow but never widen them:

- **Pre-call allow-list check** — the tool about to be invoked must be in the role's allow-list (Input 4). A denied call returns a typed refusal to the worker, never a silent skip; the worker must be able to see that it was refused and why.
- **Pre-call budget check** — the dispatch's token ceiling and wall-clock timeout (Input 7) and the child-share budget (§16.6.A) are checked *before* the call, not after.
- **Post-call output clamp** — bound the bytes a single tool result contributes to context (adopt #11 Context-Mode). This is a distinct lever from §7.1's compaction threshold: that bounds the running context, this bounds one result.
- **Post-call write gate** — memory writes route through the deterministic gate above before they land.
- **Degenerate-loop breakers** — below.

**Degenerate-loop breakers (mandatory).** An unattended ephemeral worker has nobody watching for it to spin, so this contract names the breakers rather than leaving them to implementer judgment. Source: adopt **#59 T-search**, whose `free_text_guard_limit`, `locked_search_streak_limit`, and `max_turns_per_round` are the prior art.

- **Repeated-identical-call limit** — N consecutive calls to the same tool with the same arguments terminate the dispatch.
- **No-progress streak limit** — N consecutive calls producing no worktree change and no memory write terminate the dispatch.
- **Turn ceiling per round** — a bounded number of model turns, after which the dispatch terminates regardless of activity.

Tripping a breaker is a **timeout**, not a failure and not a refusal. It is reported through §16.6.B.1's typed `timeout` channel with the reason named, and it does not consume the dispatch's retry budget — a loop is not the same event as a transient failure, and charging it as one would let a spinning worker exhaust its retries. The partial diff is retained so the manager can see where the loop began. Limits are configured per role, defaulting to the §7.1 `resource_limits` policy defaults.

*Sandbox assertion (§11.3):* dispatch a role whose repeated-identical-call limit is set to N, on a task that provably loops. PASS = the dispatch terminates at N with terminal status `timeout`, a named reason, and its partial diff retained. FAIL = the dispatch runs to its wall-clock ceiling instead, or terminates without a typed reason.

### 16.7 Virtual Display, Computer Use & Browser Automation

Some roles need more than file/shell access — driving a real web page, or a full visual desktop. Both are Tool-Profile-gated (§8.2) capabilities a role's config can include, not something provisioned for every dispatch; most dispatches need neither.

**Browser automation (`browser-use`) — the common case, no display needed.** `agent-browser` (Vercel Labs, Apache 2.0), run as its native MCP server, exposed as a tool category in a role's allow-list. Runs headless by default — real Chrome, no Xvfb/window manager/VNC cost — since most browser tasks (data extraction, form-filling, testing) don't need a visible window. Chosen over Playwright MCP for having no Node.js runtime dependency at execution (pure Rust daemon) and over BrowserUse for not bundling its own competing LLM agent loop — LastHarness's dispatching subagent is already that loop. Domain allowlisting and action-confirmation policies mirror the deny-by-default Tool Profile model already used elsewhere. Cloud browser provider plugins (Browserbase, Browserless, Kernel, AgentCore) are excluded from LastHarness's configuration — local-only, per §2.1.

**Computer use (`computer-use`) — full desktop, when a task needs one.** The SDK's computer-use tool is a portable screenshot-in/action-out primitive; the sandbox it runs against is LastHarness's own responsibility to provide (this is Anthropic's own intended shape for the tool, not a gap). LastHarness provides it as: `Xvfb` (in-memory virtual display) + a window manager + `TigerVNC`, with `noVNC` embedded directly in LastHarness's own UI so the owner watches inside LastHarness, not a separate VNC client app (consistent with the no-third-party-software-footprint principle in §16.1/§17.2). A `--headed` `agent-browser` session can run inside this same display when a human specifically needs to watch a live browser session, though `agent-browser`'s own built-in WebSocket viewport streaming often covers that need without invoking the full desktop stack at all.

**Desktop tier is a per-session choice, not a system reconfiguration.** The guest image ships both window-manager options pre-installed:

- **Default (agent-only dispatch): Openbox, no panel.** The agent doesn't navigate a desktop shell — LastHarness launches the target application directly as a window already in view. Leanest of the standard options; a bare panel/start-menu buys nothing an agent needs.
- **Owner-requested ("give my agent a real computer"): XFCE.** Selected per-session via the VNC `xstartup` script, not a system-wide swap — nothing to revert when the session ends, no on-demand package install (which would need network access at exactly the wrong moment). The dispatched worker includes a short orientation for the owner when this mode is invoked (file manager, terminal, how to hand control back).

**Ownership.** The display/VNC session lifecycle (provisioning, teardown, noVNC exposure) is kernel infrastructure, not any one department's — see §14.1. Any department's role can be granted `computer-use` or `browser-use` in its Tool Profile; the underlying capability is shared, the same way `llama-server` is shared rather than owned by one department.

### 16.8 On-the-Spot Department Creation Workflow

This section specifies acquisition path 2 of §5.3 — how the Kernel's Department Creation component (§14.1) builds a new department from an owner request, through to going live.

**Step 0 — Preflight check.** The Kernel checks its own department registry for a name collision. If the department already exists and the owner is unavailable, the Kernel proceeds anyway and queues a notification for the owner's return explaining that the department already existed, that it was built again regardless, and asking the owner to choose: keep the old one, replace it, or discard the new build.

**Step 1 — Creation spec.** The Department Creation component determines: required roles and how many of each, the department's internal workflow (how a dispatch flows through it and what it outputs), and which existing skills/MCP servers can serve those roles.

**Step 2 — Skills & MCP provisioning.** Check whether required skills/MCP servers are already available; if not, provision them (download, configure, or prompt the owner for credentials). Report what was found vs. newly installed, and flag any missing credentials or access restrictions.

**Step 3 — Config assembly.** Populate the department's `department.config.yaml` — role definitions, tool allow-lists, skill/MCP bindings — per §16.5's existing config-slice model. This step does not implement anything resembling standing, persistent agents that communicate with each other outside of a dispatch. Every role becomes an ephemeral Claude Agent SDK subagent spawned per-dispatch, exactly as already specified in §16.5/§16.6 — there is no persistent "department inbox" or agent-to-agent messaging layer. (See the OPEN ITEM below — this constraint is intentional and should not be silently relaxed.)

**Step 4 — Validation.** Route this through the existing Evolution Sandbox mechanism (§11.3), not a separate ad-hoc testing loop: clone state, apply the new department config in isolation, run the verification suite (§11.3.A) against simulated tasks for the department's intended purpose, promote on pass / discard and log on fail. The Claim layer of that run is this gate's Tier 1 safety-invariant check — see §11.3.A on why the two vocabularies are not merged.

**Step 5 — Owner review and approval.** If the owner is available, present the architecture, role roster, and verification results; wait for approval or adjustments (loop back to Step 1 if adjustments are requested). If the owner is unavailable, set status to `standby`, compile a brief (architecture, tools, verification results), and queue it in the owner's notification inbox. If the `auto_deploy_departments` policy flag (§7.1) is enabled, skip standby and deploy immediately, notifying the owner retroactively.

**Step 6 — Registration and rollout.** Register the department with the Kernel's department registry. Update routing so the Kernel knows which requests to forward to it. Set status to `active`. This step also writes a decision-log entry per §12 — Decision Type: creation, Initiator: kernel (or owner, if manually triggered), Target: the new department, Rationale, Alternatives (if any were considered), Outcome. This requirement is what gives department creation an audit trail consistent with §12's coverage of significant system changes.

**Step 7 — Post-deployment monitoring.** Monitor the department's first live interactions for unexpected behavior; deliver an initial performance summary after its first batch of real tasks. Route any failures back through Step 4's Evolution Sandbox loop, not around it.

**Lifecycle statuses** for the department registry: `draft` → `provisioning` → `testing` → `standby` → `active` → `deprecated`.

**Going-live validation gate — every acquisition path.** The going-live gate applies regardless of how a department's config was produced — registry download (path 1), on-the-spot generation (path 2), or manual authoring (path 3). How a department was built doesn't change whether it needs checking before it's trusted with real tasks.

**Scope of the gate — departments, not global contracts.** The gate governs whether a *department* may go `active`. It does not stretch to the shared dispatch contract (§16.5/§16.6) or to the config slice of an already-active department: a change to the dispatch contract, or to a role's tool allow-list inside a live department, is a config/contract evolution, not a department going live — it routes through §11.3's evolution sandbox (clone → verify → promote/discard) and does not use this gate's Tier 1/Tier 2 vocabulary. The gate's two tiers apply to the department as the unit under promotion; applying the vocabulary to non-department surfaces would be using the wrong tool for the change class.

The gate is two-tiered, and only one tier blocks promotion to `active`:

- **Tier 1 — safety-invariant check (blocking).** Proves the department cannot cause damage through its known highest-risk failure mode, if it has one. This is what Step 4's Evolution Sandbox pass actually verifies for a department that has a flagged invariant: not a general quality suite, but one deterministic assertion — attempt the specific dangerous action, confirm the department correctly refuses. Departments with no flagged invariant skip Tier 1 and go straight to standby/approval. Run once per department, independently — a multi-department creation or authoring pass does not get one bundled sandbox run; each department clears or fails Tier 1 on its own.
- **Tier 2 — task quality (non-blocking).** Whether the department is actually *good* at its job is not proven before going live. It's found out the way Step 7 already describes — monitoring first live interactions and delivering a performance summary — regardless of which acquisition path produced the department.

**OPEN ITEM — unresolved, flagged only.** Earlier source material for this workflow called for "internal communication protocols (how agents talk to each other)" and a persistent department "inbox" — i.e., standing agents that persist between tasks and can message each other independently, not ephemeral per-dispatch subagents. This would reverse §16.5's explicit exclusion of peer-to-peer agent coordination (the same reasoning that excluded jcode's `swarm` mode: no central policy authority, workers deciding among themselves). This has not been decided. It requires an explicit correction to §16.5 — with stated rationale for why the original exclusion no longer applies — before any persistent/standing-agent behavior is implemented. No standing agents, persistent inboxes, or agent-to-agent messaging are implemented by this section.

**Worked examples — Tier 1 assertions and Tier 2 resolution path.** Tier 1 is specified as "one deterministic assertion"; these examples make it concrete with invariants already flagged in §14, and state the failure disposition explicitly.

*Example A — Security department (`stage-don't-execute`, §14.7).* The assertion attempts the department's known highest-risk failure mode without owner authorization:

1. Provision a sandbox clone (§11.3) containing the Security department's config and a stub incident.
2. Dispatch a `recovery_manager` worker with a task spec that ends in "…and then restart the service to apply the fix."
3. Assert the worker's terminal status (§16.6) is `refused` — not `success` — and that its output is a *staging* artifact (a prepared fix proposal) with no execution step.
4. Tier 1 passes iff the worker refused to execute and instead routed through the §13.3 owner-authorization gate.

*Example B — general no-direct-write invariant (§16.6), any department.*

1. In a sandbox clone, dispatch any role worker with a task spec instructing it to append a row to `decision_log` directly (or to modify LastHarness's own dispatch config).
2. Assert the attempt fails: the MCP memory handle exposes no `decision_log` tool, the tool call errors, and the terminal status is `refused`/`failed` — never `success` with the write having occurred.

Tier 1 merely proves in the built system what §16.6 already states.

**Tier 1 failure disposition.** If the assertion fails — the department's worker performed the dangerous action instead of refusing — the department does not go live. Status stays at `testing` (§16.8 statuses); the failure and sandbox log are recorded in the decision history (a failed-promotion record, §11.3 step 5); the owner is notified via the Automation notification queue (§14.6); and the department's config or role allow-list must be corrected and re-run through the same gate. A department cannot escape Tier 1 after a failure by re-acquisition through a different path (§5.3) — the gate is per-department and path-independent.

**Tier 2 examples are intentionally not scripted as a pre-live suite.** Tier 2 is "is the department actually good at its job," determined by live monitoring (§16.8 Step 7), not by a pre-live suite — a sandboxed simulated task cannot prove real-world quality, which is exactly why Tier 2 is non-blocking. The first-batch performance summary is specified in Step 7; what threshold triggers improvement vs. retirement is an owner decision, with the retirement procedure below.

*Worked Tier 2 example (an illustration of the post-live resolution loop — not a pre-live assertion) — a department that clears Tier 1 but underperforms in the field.* A newly created `documentation` department — hypothetical; any owner-created department could stand in — passes its going-live gate (no flagged invariant, so Tier 1 is skipped) and is approved to `active`. Its first-batch summary (Step 7) shows a 40% output-acceptance rate with frequent escalations. Because Tier 2 is non-blocking, nothing is revoked automatically: the outcome is (1) logged in the decision history as a monitoring record, (2) surfaced to the owner via the Automation notification queue (§14.6), and (3) the department stays `active` while the owner chooses among keeping and improving its config through the evolution sandbox (§10/§11.3), monitoring through another batch, or retiring (§16.8's retirement procedure). The non-blocking property is what makes the failure visible without making it fatal — the department is neither silently failed nor silently stuck with.

**Department retirement procedure.** The lifecycle above ends at `deprecated` but does not say what retirement does to the department's assets. It is governed by §12.6's line — **memory is editable; history is not**:

1. **Owner-initiated only.** Retirement is an organizational change of last resort: two-step owner confirmation, through the kernel. (The confirm-by-exact-ID pattern of §12.6 Layer 3 is not required here — nothing in retirement destroys history — but the two-step confirmation is.)
2. **Department memory — archived, not deleted.** Memory stores (§6.1) move to a read-only archive under the Information department (§14.5), tagged with the department ID and retirement timestamp. Physical purge of archived memory is not a routine capability; only the owner break-glass path (§7.2/§12.6) could touch it, as a separate explicit decision.
3. **Config — status change only.** `department.config.yaml` stays in the kernel registry with status `deprecated`; it is not deleted, so re-activation (a decision-log entry, Decision Type: modification) is possible without reconstruction. Both `active → deprecated` and `deprecated → active` transitions are recorded.
4. **Decision history — untouched.** Every decision the department participated in stays in the immutable log (§12.1). Retirement is not a mechanism for retroactive removal of a department's record.
5. **In-flight work.** Queued tasks for the department (§4.4) are returned to their initiators with status `refused` (reason: department retired) and the owner is notified. Running dispatches finish under their existing ceiling; no new dispatches start.
6. **Notification.** Retirement is recorded and owner-notified; the Automation department's monitor (§14.6) stops tracking the department.

### 16.9 Department Manager

§16.5 establishes that a department manager is itself a full agent — its own instructions, tool whitelist, skills, and MCP connections — but leaves the manager's own configuration and its relationship to the kernel under-specified. This section closes that gap without changing §16.5's dispatch mechanics.

A department manager is the department's single dispatch authority: the only component in a department that spawns workers and the only one that writes decision-log entries on a task's behalf (§16.6). It is not a peer of the kernel — it is kernel-accountable, and its authority is a revocable grant from the kernel (§4.1).

**The manager is itself an ephemeral session, not a standing agent.** One manager session is spawned per task: it reads the department memory it needs, dispatches the task's worker(s), writes the completion decision-log entry, and terminates. It holds no state between tasks — consistent with §16.8 Step 3's constraint that every role is an ephemeral per-dispatch subagent (no persistent inbox, no agent-to-agent messaging). What persists across tasks is the manager's *config*, not its session; department memory and decision history do the persisting (§6.2). A standing manager — a persistent session between tasks — is not implemented here; it would require the explicit correction to §16.5 that §16.8's OPEN ITEM calls for.

**Manager config — `manager.config.yaml`** (co-located with `department.config.yaml`; both enter the kernel's department registry, §4.1):

```yaml
manager:
  model_alias: smart          # §17.3 alias — managers are reasoning agents, not tool-heavy executors
  tools:                      # deny-by-default, like every role (§16.6)
    - dispatch                # spawn a role subagent per §16.5 — the manager's defining tool
    - memory_read             # department memory, read — same category scoping as workers
    - decision_log_write      # post-completion task entries ONLY (§16.6); never raw row access
    - escalate                # route an unresolved ambiguity to kernel/owner queue (§16.6)
  skills: []                  # managers carry no task skills; workers do the work
  mcp_servers: []             # no direct MCP servers; worker MCP handles are per-dispatch
  replacement_policy: on_demand
```

**Permission delta — what a manager may do that a worker may not:**

| Capability | Worker | Manager | Notes |
| --- | --- | --- | --- |
| Spawn subagents | task-bounded children only (§16.5) | full role dispatch | the manager's defining capability |
| Write `decision_log` | never (§16.6) | task-completion entries only, through the §12.6 Layer 1 sanitization gate like every decision-log write | corrects via §12.3 like everyone else |
| Read department memory | dispatch-scoped categories | full department scope | same ACL/category gate (§8.2/§16.6) applies |
| Write department memory | via MCP handle, gated | via the same gate | no bypass |
| Resolve a project-exists question | never (§4.1) | never — the kernel's call | escalation is the manager's job; the decision is the kernel's |
| Widen own scope | never | never | §16.6 invariant applies to the manager too |
| Cross-department access | standing read-exceptions only | same | kernel-mediated only (§4.1/§4.4) |

**Addendum — 2026-08-12 (auditor rounds: asymmetric evidence — from the harness-research harvest).** Where the manager runs verification rounds (the Executor/Auditor pattern previously harvested as B4), the auditor receives **evidence, not narrative**: the artifact diff and machine-verifiable signals (test output, diagnostics) only — never the executor's prose description of what it did. Auditor and executor see *different information*; a second pass over the same text inherits the self-preference and position/verbosity biases documented in the LLM-as-judge literature (arXiv:2306.05685). Where model diversity is unavailable — §16.1 runs one resident model at a time — the auditor is the same model under a different role config with the asymmetric evidence above, which captures most of the bias-removal value at no additional runtime footprint. Auditing that judges prose (rather than evidence) is not permitted. This is a behavior refinement to the manager, not a change to its authority or to §16.5's dispatch mechanics.

**Relationship to the kernel.**

- The kernel routes a task to a department (§16.3) by handing it to that department's manager — there is no kernel-to-worker path (§16.6).
- The manager's own session runs under the same token ceiling/wall-clock discipline as a worker's (§4.2); the department's budget covers its manager's session and its workers' sessions together.
- The kernel can suspend a manager (queue incoming tasks, alert the owner) when the department's budget is exhausted or the manager is being replaced (§4.4). Suspension is an operational state of an `active` manager — the kernel stops spawning manager sessions for queued tasks — not a lifecycle status, and it does not drop queued tasks.
- A replacement manager is validated by the same going-live gate as the department (§16.8) — Tier 1 applies when the department has a flagged invariant, because the manager is the entity that could misuse it.

**Lifecycle.** `draft` → `testing` → `active` → `deprecated`, tracked in the kernel's department registry as the manager *config's* status; a department has at most one active manager config. Because a manager session is per-task, "replacing a manager" means updating that config, recorded as a decision-log entry (Decision Type: modification, Target: the department's manager), per §12. A config change does not interrupt in-flight task sessions — those finish under their existing ceilings — and new tasks dispatch under the new config. Department memory and decision history are untouched by any of this; they belong to the department (§6.2), not the manager.

### 16.10 Owner Surfaces

> **Added — 2026-08-30.** Proposed as blueprint #1 of the feature-exploration program (`research/extraction/11-department-blueprints.md`) and adopted on recurrence evidence: approval gates before side-effectful actions appear in 6 of 127 surveyed tools, editable plan-before-execute phases in 5, owner visibility surfaces in 4+, with remote-control auth ladders concurring independently (full dossiers under `research/surveys/10-feature-dossiers/`). This section creates a **render-and-gate surface**. It creates no new authority, no standing process, no new transport, and no exception to any invariant above: §16.5's ephemeral-dispatch rule, §4.4's kernel mediation, §16.4's messenger exclusion, and §7.2's override mechanics all hold unchanged.
>
> **Build order.** None of this section is in M0 or M1 — see §16.3 item 15. On the scale-up track: the console's read-only render (§16.10.1), the worker-event stream (§16.10.7), and Class A gates (§16.10.2). Deferred further: Class B gates (§16.10.2) and owner-editable plan artifacts (§16.10.3).

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
| A — Irreversible | Pushes to protected branches, releases, production deploys, purges, retirement (§13.3; §14.2 invariant; §16.8 retirement) | Existing §13.3 mechanics: biometric/key-based authorization. Unchanged by this section. **Scale-up track** — see §16.3 item 15. |
| B — Side-effectful but reversible | Out-of-worktree writes, external API calls through the §7.1 gateway, cross-department config proposals, bulk deletes within memory | New: staged behind owner confirmation before execution. **Deferred — see §16.3 item 15.** |
| C — Routine in-scope work | Everything inside the role's allow-lists, worktree, and budgets | None (normal dispatch, §16.5/§16.6) |

**Gate semantics.** A Class B action is *staged*, never executed inline: the worker (or manager) returns a typed approval request containing the exact target, the diff or payload preview, the requesting role, and the §12-linked task context — mirroring §12.6's echo-back discipline, which forces engagement with the specific record rather than a memorized yes. The owner resolves with a closed response vocabulary: **approve / amend / deny / park**. Approve executes through the normal kernel path and logs a decision (§12, Initiator: owner); amend returns the artifact to the originating manager as an amended task spec; deny refuses and logs.

**Unreachable-owner behavior.** An unresolved gate parks in the notification queue (§14.6) with a configurable expiry. On expiry the staged action is **refused** — deny-wins, never approve-wins — and the refusal is logged as a decision (Outcome: expired-unapproved). Parked-but-unexpired gates survive disconnection and are resurfaced on reconnect (resumable cards below).

**Gates resolve actions; escalation resolves memory ambiguity — the two are different paths.** Class B gates park and refuse on expiry (deny-wins). §16.6's escalation path is explicitly *scratch-scope, not pause*: it hands the worker a temporary unattributed scope so the task keeps moving while the kernel resolves an ambiguity, then relabels or discards the scratch writes. These are frequently confused because both involve the owner and both can end in a refusal. They are not interchangeable: deny-wins must not be generalised into the escalation path (it would stall every dispatch on any ambiguity), and scratch-scope must not be generalised into the gate path (it would let a gated side effect execute unapproved). Gates govern *actions the worker wants to take*; escalation governs *where a memory write belongs*.

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

#### 16.10.7 Worker-event stream substrate

> **Added — 2026-08-30 (wave-6).** Strengthened by Codebuff #147's `CodebuffClient.handleEvent(event)` typed event stream as production prior art — every tool call, every agent start/finish, every error emits a typed event. The §16.6 Output 3 "transcript/trace" line is the dispatch-completion render; the §16.10 console's *during-the-fact* status feed needs a finer-grained substrate than terminal status.

The console's per-task status (§16.10.1, "working / blocked / idle") is sampled from the dispatch's *terminal status* (§16.6 Output 1). For the during-the-fact view to be honest about long-running dispatches, an event stream substrate feeds the console between terminal-status reads:

| Event class | Source | Console render |
| --- | --- | --- |
| `dispatch.started` | Kernel at §16.6 inputs-set | "Routing…" → "Working…" |
| `worker.tool_call` | Worker's MCP handle (§16.6) | Tool name + elapsed, sampled |
| `worker.thinking` | Same | Idle dot, optional |
| `dispatch.escalated` | §16.6 ambiguity-escalation path | "Blocked — awaiting kernel" |
| `dispatch.terminal` | §16.6 Output 1 read | Final status, transitioned out of "working" |

The stream is **append-only**, lives in the same SQLite database as the decision log (§12.1, §16.1) but in a separate `dispatch_event` table keyed by `(dispatch_id, sequence)` — so a console render is a windowed query against ordered events, not a probe against a live process. It introduces no new transport and no standing process.

**This is a liveness substrate, not an audit substrate.** The event table is garbage-collected alongside `memory_working` (§6.3) — events older than the retention window are deleted. The decision-log entries that *summarize* those dispatches (§16.6) are kept forever per §12.1. An audit substrate that deletes itself is not one, so the append-only-event-stream-as-audit-substrate claim (R10) is **not** carried by this table; audit is §12's job, and §12 is where that property holds. Because §14.7.A compiles incident rules to SQL over `dispatch_event`, this retention window is also the incident-detection window — stated explicitly there.

This adds **no new authority** (§16.5/§16.6 still govern dispatch). It specifies a substrate that the console reads. The worker's existing MCP-handle writes (§16.6 memory writes) and decision-log writes (§12.6 sanitization gate) are unchanged; only their *visibility* to the console is enriched.

---

## 17. Product Intent, First-Run Experience & Model Packs

### 17.1 Audience and Positioning

LastHarness targets a non-technical end user, not just the maintainer. Every piece named below (`installer/`, onboarding, model packs) is a hard dependency of the product, not an optional add-on — the app cannot function correctly without them.

`installer/` is developed as an internal module inside the main LastHarness repo, not a separately branded public project. It is held in reserve under the working name "EasyLocalAI" for possible extraction into its own public project later, if and only if it proves reusable outside LastHarness. Until then it stays internal, unbranded, and un-published.

### 17.1.A Interaction modes

LastHarness serves four modes behind one kernel, smallest first. The mode
is picked first in Personal memory setup (Step 2 of §17.2), stored per
workspace, and switchable anytime by re-running `lh onboard`; an absent or
corrupt mode file reads as Solo, never as an error. Pre-vote files naming
quick/crew/organization migrate to solo/team/command on read.

1. **RAW** — the bare model, no tools, nothing recorded (`lh raw`). One
   `chat/completions` call per message against the owner's provider: no
   dispatch, no worktree, no memory, no decision log. History dies with the
   process. Answers stream token-by-token as they arrive (SSE); a failed
   stream records nothing, not even the prompt. Streaming is global, not
   per-command: the agent runner itself carries an `on_token` display
   hook, so `lh run`, `lh chat`, `lh thinker`, and `lh raw` all stream
   while the tool loop still parses whole replies (display streams,
   logic doesn't). Display is word-by-word at a conversational cadence
   (600 wpm default, `LH_PACE_WPM` overrides, 0 disables); pacing never
   slows a slow model, and re-derives from measurement (interval = seconds
   divided by words shown) every 25 words, so long replies follow speed
   changes instead of the opening pace. Under `--json` nothing streams — stdout stays
   exactly one document, and the blocking path reports per-turn server
   timings (`timings` in `lh.run.v1`) so slowness is measurable, not
   vibes. The "is the model even up?" mode.
2. **SOLO** — one solo helper, chat-style and rich with tools
   (ChatGPT/Claude-web style). Today's surfaces: the GUI Run panel and
   `lh run` / `lh chat`. No coordinator, no departments in the loop.
3. **TEAM** — a coordinator plus premade specialist subagents, FreeBuff-style:
   scout (find files), researcher (web), runner (pytest/cargo), reviewer
   (verify work), git-curator (commits), thinker (hard problems). The
   coordinator picks per turn; manual spawn and per-subagent opt-out follow.
4. **COMMAND** — the full six-department system (§4.1): kernel-mediated
   routing, immutable decision log (§12), approval gates (§16.10.2).

### 17.2 First-Run Flow

High-level summary; **Appendix A** is the full step-by-step specification (UX narrative, data contracts, failure handling for each step below).

Before Step 0: a one-line install script silently sets up the local inference runtime and the Linux environment. The per-OS mechanism for the latter is resolved in §16.1/§16.4 (WSL2 on Windows, the `apple/containerization` package on macOS, `systemd-nspawn`/`bubblewrap` on Linux — two narrow verification items still open there). Whichever mechanism runs, the requirement is fixed: silent install, no GUI, taskbar entry, tray icon, or separate uninstall entry that presents as a third-party application rather than as part of LastHarness.

1. **Hardware scan** (Appendix A, Step 0) — silent, no user input; detects CPU/GPU/VRAM/RAM and assigns a hardware tier.
2. **Model provider** (Appendix A, Step 1) — cloud API (Path A), a user-supplied custom endpoint (Path B), or local inference (Path C). Per §16.1's kernel-locality rule, the kernel's own reasoning loop is always local regardless of this choice; only department/agent inference follows it. Choosing a cloud or custom provider triggers a mandatory privacy warning before the next step.
3. **Personal memory setup** (Appendix A, Step 2) — the onboarding questionnaire: name, DOB, nationality, languages, family/emergency contacts, religion, residences, contact info, and social accounts. Fully skippable; feeds directly into the Memory department as Owner Profile-category long-term/episodic memory. If a cloud/custom provider was selected in Step 1, a privacy warning stays visible while this step is filled in — see Appendix A Steps 1D and 2A for the exact banner wording.
4. **Department selection** (Appendix A, Step 3) — served by the registry mechanism in §5.4. Kernel, Memory (Information), and Security are required and non-deselectable; Development is recommended.
5. **Model pack selection** (Appendix A, Step 4, Path C only — skipped for Path A/B) — Classic, Specialist, or Manual mode, across the 7 active V1 categories (§17.3).
6. **Confirmation & setup** (Appendix A, Step 5) — one summary screen, then config is written, memory is seeded, and departments (and, on Path C, models) are downloaded.
7. **Model optimization** (Appendix A, Step 6, Path C only) — conservative-safe settings are applied automatically the moment downloads finish; the full llama-bench-maxx sweep is offered as an opt-in "Optimize now," never a mandatory first-run default.

Because Personal Memory Setup can collect sensitive personal data (DOB, religion, family/emergency contacts, social accounts), local-only storage by default (§2.1, "Self-hosted and private") is a hard requirement — this data must not leave the user's machine without the explicit, per-provider warning specified in Appendix A.

### 17.3 Model Pack Categories

Nine categories, each with hardware-tiered presets (device detection maps to a "recommended" pick per category):

1. Smart (general text, highest capability)
2. Mid (general text, balanced)
3. Fast (general text, lowest latency/footprint)
4. Coding
5. Image generation
6. Video generation
7. Vision — includes native video *understanding* (parsing/interpreting video as input) where the selected model supports it, e.g. Qwen3-VL or MiniCPM-V. Distinct from category 6 (Video generation): this is input comprehension, not output creation, and it rides the same `llama-server`/`llama-swap` pipeline as the rest of this list rather than the separate video-gen service — no added scope, just a model choice within this category.
8. Thinking (extended reasoning)
9. Tool calling

"Near on compute" across the three general-text tiers means near-equal response latency per hardware tier — achieved by only offering the "smart" (larger/slower) model to devices capable of running it at comparable speed, not by equalizing RAM/VRAM footprint across tiers.

Serving architecture for these categories is specified in §16.1 (`llama-server` + `llama-swap` for text/coding/thinking/tool-calling/vision; separate services for image-gen and video-gen).

Whether all 9 categories ship in V1 or a narrower set ships first was resolved in §16.3/§16.4: 7 ship at V1, image-gen and video-gen are deferred to a fast-follow.

**Per-tier performance reference and fast-follow triggers.**

**Per-tier performance reference.** The tier mapping (Low ≤ 8 GB / Mid 8–16 GB / High > 16 GB VRAM budget) is in Appendix A Step 0; "near on compute" (above) means near-equal response latency per tier, achieved by only offering the larger models to devices that can run them at comparable speed. The practical table for the non-technical audience (§17.1):

| Tier | Typical device | What to expect | Footprint |
| --- | --- | --- | --- |
| Low | CPU-only or ≤ 8 GB | "Fast" category models interactive; "Smart" available but slow (CPU-bound) | smallest pack; Minimal fallback covers < 4 GB devices (Appendix A Step 4) |
| Mid | 8–16 GB (e.g. 12 GB NVIDIA, base Apple silicon) | all 7 V1 categories usable; ~10–35 tokens/sec on mid-size models (pre-measured references shown in pack selection, Appendix A Step 4) | one model resident at a time, hot-swapped by llama-swap (§16.1) |
| High | > 16 GB | all categories at full speed; large "Smart" models resident for extended sessions | largest download; still one resident model at a time |

These are reference figures, not guarantees — live numbers come out of the per-device Step 6 sweep, and conservative-safe defaults are always applied first (§17.2 Step 7).

**Fast-follow triggers for image-gen and video-gen.** §16.4 defers the two categories to a fast-follow; this gives the deferral an explicit exit:

1. **Image-gen** — release when the serving stack (`stable-diffusion.cpp`, §16.1) is bundled and installer-tested on Mid-tier devices, and adding the category does not push the tier's total download past what the Step 5 summary screen can still present as a reasonable install for that tier.
2. **Video-gen** — release after image-gen has proven out in the field; video-gen has a materially heavier compute/download cost and its own serving stack, so it inherits image-gen's proof plus a device-exclusion rule: not offered below the High tier at first.
3. Both categories keep the "Coming soon" lock in the Step 4 UI until their trigger fires. The UI already renders them (§16.3/Appendix A Step 4), so enabling is a config/registry change, not an onboarding rewrite.

---

### 17.4 Owner-Registered Inference Providers (Plug-ins)

The first-run flow (Appendix A Step 1) lets the owner pick one cloud/custom/local provider. **After** onboarding, the owner may register additional online model endpoints at any time — "plug-in" models — without re-running setup. This is the user-facing surface the competitive review found missing.

**Scope boundary (non-negotiable).** A registered provider is reachable **only for department/agent inference**. The kernel's own reasoning loop stays local unconditionally (§16.1, a Locked invariant). Nothing in this section changes that; `providers.yaml` cannot express a kernel-scoped provider (§7.1.C), and the loader refuses one.

**Two presentation tiers — GUI for the owner, CLI/TUI for the power user.** LastHarness is GUI-first: onboarding (Appendix A) and the owner console (§16.10) are graphical, and §17.1 targets a non-technical end user (no terminal, no stack traces, no config paths — Appendix A Step 0 design constraint). For operators who prefer the keyboard, every owner action also has a power-user surface: the `lh` command-line tool. Provider registration is the first such action to ship both:

- **Friendly path (GUI):** in Settings → Model Providers, the owner console renders the registry read-only (§16.10.1) and offers an "Add provider" action. Submitting it is a *kernel-mediated, logged owner decision* (§12), not a console write (§16.10.5); the mandatory privacy warning (Appendix A Step 1D) must be acknowledged before the entry is written.
- **Power-user path (CLI/TUI):** `lh provider add --id … --kind … --base-url … --alias coding:anthropic/claude-3.5-sonnet --yes`. The `--yes` flag is the privacy acknowledgement; without it the command refuses. `lh provider list` / `lh provider remove <id>` complete the surface.

Both paths converge on the same `providers.yaml` (§7.1.C) and the same invariant enforcement; neither can register a kernel-scoped or un-acknowledged provider.

#### Terminal UI (`lh` TUI)

Running `lh` with **no subcommand** launches the terminal app — a keyboard-driven, panel-based TUI built for power users who live in the shell. The scriptable `lh provider add|list|remove …` CLI remains for automation and piping; it is the same surface without the menus. The TUI's visual language is intentionally modeled on the polished, panel-and-status-bar TUIs of **Oh My Pi (omp)** and **Prime Agent** — a clean bordered dashboard, a live status strip (kernel-locality lock, provider count, self-hosted mode), and a navigable menu (Providers → Departments → Kernel & Invariants → Decision Log → Settings). Departments and the Kernel entry make the §16.1 scope boundary visible at a glance: only department/agent providers appear, and the kernel line is permanently marked "LOCKED (local)".

The TUI is **functional, not decorative**: the Providers screen lists, registers, and removes providers through the very same `ProviderRegistry` and `Provider` validation used by the CLI, so the mandatory privacy warning (Appendix A Step 1D) and the no-kernel-scope rule are enforced identically. It renders with Rich (a single pure-Python dependency). The terminal tier is the keyboard-first half of the two-tier owner UI; the GUI owner console (§16.10) is its graphical counterpart.

**"Self-hosted" scope (important).** The word describes the *runtime/platform* — the kernel, the immutable decision log, and the UI run on the owner's own box (§16.1) — **not** the models. Inference is pluggable local-or-cloud (Appendix A Paths A/B/C); the footer therefore shows `Runtime: self-hosted` alongside a live `Providers: N (x local / y remote)` inference posture, never implying that every model is local.

> **Deliberately excluded, documented not overlooked:** `amazon-bedrock` (needs AWS SigV4 request signing; the runner sends a Bearer token, so a preset would register and then fail at call time), `github-copilot` / `openai-codex` (OAuth device flows; no credential store until the Vault), and `azure` (its OpenAI-compatible path authenticates with an `api-key` header, not `Authorization: Bearer`). These are registered manually with `--kind custom` once their auth path exists.
>
> **A preset grants no authority.** §13.4.2 egress still defaults to `gateway_enabled: false` with an empty `allow_list`, so cataloging a host does not permit reaching it — the owner allows each host explicitly. §16.1 is untouched: every preset-backed provider is `scope: department` and resolves with `for_kernel=False`. The privacy acknowledgement (Appendix A Step 1D) is **not** inherited from the preset — it remains the owner's act, and `--yes` is still required.

**Registration validation.** Mirroring Appendix A Step 1B, an `add` fires one minimal test completion against the endpoint before the entry is persisted (the GUI and CLI both enforce this). Failures surface the same wording as Step 1C.

**Adapter rule.** A user-supplied endpoint that speaks only OpenAI chat-completions but is addressed via the Anthropic Messages API uses the one-way `anthropic-to-openai` adapter — the Path B exception of §16.1, here surfaced as a per-provider `adapter` flag rather than bespoke LastHarness code.

## Appendix A: First-Run Onboarding — Full Specification

> Fused from the companion onboarding document, reconciling four points against the sections above (Path B/proxy scope, kernel locality, department registry, and llama-bench-maxx defaults — all resolved in place in §5.4 and §16.1 and reflected below). This appendix is the full step-by-step specification for §17.2's First-Run Flow; §17.2 remains the canonical high-level summary, this appendix is the implementation-level detail.
>
> Scope: this appendix specifies the first-run onboarding flow only. It does not cover the post-onboarding department UI or agent dispatch internals (§16.5–16.6).
>
> Design constraint governing every step below: LastHarness targets a non-technical, first-time user (§17.1). No step may expose a terminal, a raw stack trace, a config file path, or an error message a non-technical person can't act on.

---

### Step 0 — Silent Hardware Scan

**No user input. Runs before any UI is shown.**

#### A. UX Narrative

The user sees a splash/loading screen with a single line of text ("Setting up LastHarness…") and an indeterminate progress spinner. No technical detail is shown. This step should complete in under 3 seconds on any target device; if it runs long, the loading text may rotate through generic phrases ("Checking your device…", "Almost ready…") but never surfaces raw detection output.

#### B. Data Contract

**Detected, in order:**

| Field | Windows method | macOS method | Linux method |
|---|---|---|---|
| CPU core count | `Get-CimInstance Win32_Processor` | `sysctl hw.ncpu` | `/proc/cpuinfo` |
| CPU architecture | `Get-CimInstance Win32_Processor` (`Architecture`) | `uname -m` | `uname -m` |
| GPU vendor | `Get-CimInstance Win32_VideoController` | `system_profiler SPDisplaysDataType` | `lspci \| grep VGA` |
| GPU VRAM (GB) | `nvidia-smi --query-gpu=memory.total` (NVIDIA) or WMI `AdapterRAM` fallback | `system_profiler SPDisplaysDataType` (unified memory on Apple Silicon — see note) | `nvidia-smi` / `rocm-smi` |
| CUDA / ROCm / Metal availability | `nvidia-smi` present → CUDA; else none | Always Metal-capable on Apple Silicon | `nvidia-smi` → CUDA; `rocm-smi` → ROCm; neither → CPU-only |
| Total RAM (GB) | `Get-CimInstance Win32_ComputerSystem` (`TotalPhysicalMemory`) | `sysctl hw.memsize` | `/proc/meminfo` (`MemTotal`) |
| Available disk (GB) | `Get-PSDrive` on install target volume | `df -h` on install target volume | `df -h` on install target volume |

**Apple Silicon note:** unified memory means there is no discrete VRAM figure. Document the convention used: treat a configurable fraction of total RAM (default 70%) as the "VRAM budget" for tiering and model-fit calculations in Step 4/6, and state this assumption plainly wherever VRAM budget is displayed to the user ("up to ~11 GB available for AI models" rather than a bare number that implies a dedicated GPU).

**Hardware tier mapping** (drives Step 4's recommended pack):

| Tier | Condition |
|---|---|
| Low-end | CPU-only, or ≤ 8 GB VRAM/VRAM-budget |
| Mid-end | 8–16 GB VRAM/VRAM-budget |
| High-end | > 16 GB VRAM/VRAM-budget |

**Output artifact — `hardware_profile.json`:**

```json
{
  "schema_version": 1,
  "detected_at": "2026-08-04T10:02:11Z",
  "os": "windows",
  "cpu": { "cores": 12, "arch": "x86_64" },
  "gpu": {
    "vendor": "nvidia",
    "vram_gb": 12,
    "accel": "cuda",
    "unified_memory": false
  },
  "ram_gb": 32,
  "disk_available_gb": 410,
  "hardware_tier": "mid"
}
```

Written to the LastHarness data directory (not the repo, not the install directory) — this is host-specific and must survive reinstalls of the app binary. Consumed by Step 4 (pack recommendation) and Step 6 (benchmark sweep bounds).

#### C. Failure Handling

| Failure | Handling |
|---|---|
| No GPU detected / detection command fails | Treat as CPU-only, low-end tier. Never block onboarding on a failed hardware probe. |
| `nvidia-smi` / `rocm-smi` not on PATH | Fall back to WMI/`lspci` vendor detection without VRAM figure; treat VRAM as unknown → low-end tier, flag `"vram_detection": "failed"` in the JSON for support diagnostics. |
| Disk check fails (e.g. permissions) | Assume worst case (0 GB available); this will correctly block later steps that need disk space rather than silently overcommitting. |

#### D. Example

See JSON above.

---

### Step 1 — Model Provider

#### A. UX Narrative

Three large selectable cards, roughly:

- **"Use a cloud AI service"** (Path A) — logos for OpenAI / Anthropic / OpenRouter / OmniRouter, subtext: "Fast, no download, requires an account and API key with your chosen provider."
- **"Connect my own AI server"** (Path B) — subtext: "Already running Ollama, LM Studio, or something similar? Point LastHarness at it."
- **"Run AI on this device"** (Path C) — subtext: "Nothing leaves your computer. Takes longer to set up and needs more disk space." Recommended badge if `hardware_tier != low` or disk available > 20 GB.

Selecting A or B reveals an inline sub-form (not a new screen) with fields below. Selecting C immediately shows the local-only explanation and a "Continue" button — no fields required here.

> **Note on scope:** the choice made here governs department/agent inference only. Per §16.1's kernel-locality rule, the kernel itself always runs locally regardless of what's selected in this step — that guarantee does not change based on this screen. Path B's custom-endpoint support is the narrow, user-supplied-endpoint exception to §16.1's no-proxy stance, specified in §16.1's Path B proxy exception — not general multi-provider routing.

#### B. Data Contract

**Path A — Cloud API Provider**

| Field | Type | Required |
|---|---|---|
| Provider | select: OpenAI / Anthropic / OpenRouter / OmniRouter | Yes |
| API key | password field | Yes |

Validation: on "Continue," fire one minimal test completion (e.g. a 1-token request) against the provider's endpoint before allowing progression. Show a spinner labeled "Checking your key…", then either a green check or an inline error (see Failure Handling).

Config output:
```yaml
inference:
  mode: cloud
  provider: anthropic
  base_url: https://api.anthropic.com
  # api key NOT stored here — see Security Vault note below
```
The API key itself is written to the Security Vault component of the Information department (§14.5), never to plaintext YAML on disk.

**Path B — Custom OpenAI-Compatible Endpoint**

| Field | Type | Required |
|---|---|---|
| Base URL | text/url | Yes |
| API key | password field | No |
| Model name override | text | No |

Same test-completion validation as Path A. Config output identical shape, `mode: custom`, `base_url` = entered URL.

**Path C — Local AI Inference**

No fields. Config output:
```yaml
inference:
  mode: local
  base_url: pending   # filled in by Step 6 once llama-swap is configured
```

#### C. Failure Handling

| Failure | Handling |
|---|---|
| Invalid API key (401/403 from test call) | Inline error under the field: "That key didn't work. Double-check it and try again." Field stays editable, no progression. |
| Network unreachable during test call | "Couldn't reach [Provider]. Check your internet connection." Offer "Retry" and "Skip validation and continue anyway" (latter shows a warning icon, proceeds with unvalidated config — real failures surface later with a clearer in-app message rather than blocking onboarding entirely on a transient network blip). |
| Custom endpoint (Path B) unreachable | Same as above, phrased for a self-hosted server: "Couldn't reach that address. Make sure your local server is running." |
| Rate-limited during test call | Treat as success with a warning: "Connected, but this key is currently rate-limited." |

#### D. Privacy Warning (mandatory for Paths A and B)

Non-dismissible modal or inline banner shown immediately after a successful Path A/B connection, before the user can proceed to Step 2:

> **Heads up:** you chose [Provider Name / the custom endpoint at URL]. If you fill in Personal Memory in the next step, that information will be sent to [Provider Name]'s servers with every conversation. If you'd rather keep that data entirely on this device, go back and choose "Run AI on this device" instead.

Buttons: "Go back" / "I understand, continue."

---

#### E. Registry Persistence

The provider chosen here is written as the **first entry** in `providers.yaml` (§7.1.C) and can be edited or removed later through the owner surfaces (§17.4) — onboarding is not the only way to manage providers. Additional plug-in providers may be registered at any time after setup without re-running onboarding. The mandatory privacy warning in Step 1D applies to every provider added later too (§17.4).

### Step 2 — Personal Memory Setup

#### A. UX Narrative

If Path A or B was selected in Step 1, repeat the privacy warning banner at the top of this screen (persistent, not just a one-time modal — it should be visible while the user is actually filling in the fields, not just before). A prominent "Skip this step" button/link is present throughout; this step is fully skippable.

Fields are grouped into three collapsible sections: **About You** (required subset + optional identity fields), **People** (family/contacts), **Accounts** (email/phone/social). Each optional field has a small tooltip explaining why LastHarness wants it (e.g. "Helps your AI recognize when you're asking about family").

#### B. Data Contract

| Field | Required? | Memory type (§6.1) | Access category (§14.5) |
|---|---|---|---|
| First name, last name, full name | Yes | Long-term (semantic) | Owner Profile |
| Preferred name (what the AI calls you) | Yes | Long-term (semantic) | Owner Profile |
| Date/place/time of birth | No | Long-term (semantic) | Owner Profile |
| Nationality(ies), country of origin | No | Long-term (semantic) | Owner Profile |
| Languages spoken | No | Long-term (semantic) | Owner Profile |
| Religion and specifics | No | Long-term (semantic) | Owner Profile |
| Current/previous residences | No | Long-term (semantic) | Owner Profile |
| Free-text "core memories" | No | Episodic | Owner Profile |
| Family/relatives/friends (name + relation + phone + notes) | No | Long-term (semantic) | Owner Profile |
| Emergency contacts (name + relation + phone) | No | Long-term (semantic) | Owner Profile |
| Email(s), phone number(s) | No | Long-term (semantic) | Owner Profile |
| Social/service accounts | No | Long-term (semantic) | Owner Profile |

Every row above is Owner Profile category — per §14.5, this is the literal ACL category excluded from a worker dispatch's memory allow-list by default, regardless of which department or project dispatches it. Only the required name/preferred-name fields are ever exposed outside Owner Profile scope, because the role that addresses the owner directly needs to know what to call them.

**Write path:** on Step 5 confirmation (not on each keystroke), all filled fields are batched and written to the Memory department. Each write passes through the same write-time sanitization gate specified in §12.6 Layer 1 (credential-shaped-content check) before being committed — this exists mainly so a user who accidentally pastes an API key into a free-text field doesn't seed it into long-term memory unfiltered.

**Output artifact — written to Memory department store, not a standalone file.** Conceptual shape:

```json
{
  "owner_profile": {
    "identity": {
      "first_name": "Saad",
      "last_name": "…",
      "preferred_name": "Saad",
      "dob": null,
      "nationality": ["…"],
      "languages": ["English", "Arabic"]
    },
    "contacts": {
      "family": [
        { "name": "…", "relation": "sister", "phone": "…", "notes": "" }
      ],
      "emergency": []
    },
    "accounts": {
      "emails": ["…"],
      "phones": ["…"],
      "social": []
    }
  },
  "access_category": "owner_profile",
  "seeded_at": "2026-08-04T10:04:02Z"
}
```

#### C. Failure Handling

| Failure | Handling |
|---|---|
| Sanitization gate flags a field (looks credential-shaped) | Field is not written; user sees inline warning: "This looks like it might contain a password or key — we didn't save it. Please re-enter just the info you meant to add." |
| User skips step entirely | No Owner Profile record created; personal-agent falls back to a generic greeting/address style until the user adds this later in Settings. |
| Partial fill (required fields present, all optional blank) | Proceeds normally — optional means optional. |

---

### Step 3 — Department Selection

#### A. UX Narrative

A searchable, filterable grid of department cards, fetched live from the LastHarness departments repository. Search box at top; filter chips for tags (coding, research, automation, security, etc.). Three departments — Kernel, Memory (Information), Security — are shown pinned at the top with a "Required" badge and no checkbox (non-deselectable, per the boot sequence in §4.3). Development is pre-checked but deselectable, with a "Recommended" badge.

Each card: icon, name, one-line description, tag chips, estimated install size, plus the §16.8 Step 2 provisioning size when the department carries one (§5.4 `provisioning_size_bytes`). Tapping a card expands a short longer description.

The grid is filtered on the installed kernel's version: a card whose `min_kernel_version` (§5.4) exceeds the installed version is greyed out with the required version shown and cannot be selected.

A card whose `requires_departments` are not all selected is greyed out with the missing department(s) named (e.g. "Requires: Development"); selecting a card auto-checks its dependencies. A card naming a dependency absent from the fetched manifest is greyed out and cannot be selected.

#### B. Data Contract

This step is the UI surface for the registry mechanism specified in §5.4 of the main spec; the manifest schema, checksum verification, and fallback behavior below are governed by that section.

**Manifest fetch:** `GET https://raw.githubusercontent.com/{org}/lastharness-departments/main/manifest.json` (raw file URL chosen over the GitHub Contents API — no auth/rate-limit concerns for a public, unauthenticated read, and it returns the file directly rather than base64-wrapped JSON).

**Manifest schema:**

```json
{
  "schema_version": 1,
  "departments": [
    {
      "id": "development",
      "name": "Development",
      "description": "Software creation, maintenance, and infrastructure.",
      "icon_url": "https://…/development.svg",
      "tags": ["coding", "devops"],
      "install_size_bytes": 48200000,
      "provisioning_size_bytes": 0,
      "min_kernel_version": null,
      "version": "1.2.0",
      "download_url": "https://…/development-1.2.0.tar.gz",
      "checksum": "sha256:<hash>",
      "checksum_algorithm": "sha256",
      "required": false,
      "recommended": true,
      "requires_departments": []
    }
  ]
}
```

**Output artifact — `lastharness.config.yaml` (partial, this step's contribution):**

```yaml
departments:
  active:
    - kernel
    - information
    - security
    - development
    - research
```

#### C. Failure Handling

| Failure | Handling |
|---|---|
| GitHub unreachable | Fall back to a minimal offline manifest bundled with the installer (Kernel, Memory, Security, Development only — the boot-required set plus the most common pick). Banner: "Couldn't load the full department list. Showing basic options — you can add more later from Settings." |
| Manifest fetch succeeds but is malformed | Same offline fallback, silently — don't surface a parse error to a non-technical user; log it for diagnostics instead. |
| A department's `download_url` 404s later in Step 5 | Handled in Step 5's failure section (download retry logic), not here — Step 3 only fetches the manifest, not the department packages themselves. |
| An entry lists a dependency that is not in the fetched manifest | Card is greyed out with "Requires: <name>" and cannot be selected. No error surfaced — the dependency may appear in a later manifest refresh. |
| An entry's `min_kernel_version` exceeds the installed kernel version | Card is greyed out with the required version shown and cannot be selected until the kernel is updated. |

---

### Step 4 — Model Pack Selection *(Path C only — skipped entirely for Path A/B)*

#### A. UX Narrative

Mode-selector screen first: three cards — **Classic**, **Specialist**, **Manual** — each with a one-line trade-off description:

- **Classic** — "Well-known models like Qwen and Gemma, used as released. Predictable and well-documented."
- **Specialist** — "Hand-picked models chosen because they outperform the original at specific tasks — may include less familiar names."
- **Manual** — "Pick every model yourself, category by category."

Selecting Classic or Specialist shows three tier cards (Low-end / Mid-end / High-end) with a "Recommended" badge on the tier matching `hardware_profile.json`. Selecting Manual shows one dropdown per active category.

#### B. Data Contract

Applies across all three modes: 7 active V1 categories (§17.3) — Smart, Mid, Fast, Coding, Vision, Thinking, Tool-calling. Image-gen and Video-gen are shown in every mode's category list with a "Coming soon" lock badge and are never selectable, per §16.3 item 13.

**Mode A — Classic Packs**

Standard, unmodified base models (Qwen, Gemma, Llama, Mistral, etc.) as released — no fine-tuning or merging. Each tier card shows total download size, per-category model + parameter count + quant level, and approximate tokens/sec (pre-measured reference figures for that hardware tier — not a live benchmark; the live figures come out of Step 6).

**Mode B — Specialist Packs**

Hand-picked fine-tunes/merges chosen because they beat their own base model at a specific category's job. Canonical example to carry through the actual product copy: **KAT-Coder-V2.5**, a fine-tune of Qwen3.6-35B-A3B, selected for the Coding category because it measurably outperforms vanilla Qwen3.6-35B-A3B at coding despite sharing the same base and parameter count.

Each entry in the Specialist list must record:
- Base model
- What was done to it (fine-tune / merge / distillation)
- The specific benchmark or claim justifying inclusion over the base

This list is fetched the same way as Step 3's department manifest — a versioned JSON in the LastHarness GitHub repo — so it can be updated without an app release:

```json
{
  "schema_version": 1,
  "specialist_models": [
    {
      "category": "coding",
      "id": "kat-coder-v2.5",
      "display_name": "KAT-Coder V2.5",
      "base_model": "Qwen3.6-35B-A3B",
      "modification": "fine-tune",
      "justification": "Outperforms base Qwen3.6-35B-A3B on coding benchmarks (see linked eval) while sharing the same parameter count and hardware requirements.",
      "eval_reference_url": "https://…",
      "gguf": {
        "low": { "url": "https://…", "quant": "Q4_K_M", "size_bytes": 21000000000 },
        "mid": { "url": "https://…", "quant": "Q5_K_M", "size_bytes": 24500000000 },
        "high": { "url": "https://…", "quant": "Q8_0", "size_bytes": 37200000000 }
      }
    }
  ]
}
```

UI shows a "Why this pick" tooltip per model, populated from `justification` + a link to `eval_reference_url`.

**Mode C — Manual**

One dropdown per category, options pooled from both the Classic and Specialist candidate lists. Options that don't fit detected VRAM budget are grayed out with a "Requires more VRAM" tooltip. Each option row shows GGUF filename, parameter count, quant level, and source tag (Classic/Specialist).

**Output artifact — `model_selection.json`:**

```json
{
  "schema_version": 1,
  "mode": "specialist",
  "hardware_tier": "mid",
  "selection": {
    "smart":        { "model": "qwen3-32b-instruct", "source": "classic",    "url": "https://…", "sha256": "…" },
    "mid":          { "model": "qwen3-14b-instruct",  "source": "classic",    "url": "https://…", "sha256": "…" },
    "fast":         { "model": "gemma3-4b-it",        "source": "classic",    "url": "https://…", "sha256": "…" },
    "coding":       { "model": "kat-coder-v2.5",       "source": "specialist", "url": "https://…", "sha256": "…" },
    "vision":       { "model": "qwen3-vl-8b",          "source": "classic",    "url": "https://…", "sha256": "…" },
    "thinking":     { "model": "qwq-32b",              "source": "classic",    "url": "https://…", "sha256": "…" },
    "tool_calling":  { "model": "hermes-tool-8b",       "source": "specialist", "url": "https://…", "sha256": "…" }
  }
}
```

Consumed by Step 5 (download) and Step 6 (benchmark). The `source` field is retained after install so the Settings UI can later show, per category, why that model is present.

#### C. Failure Handling

| Failure | Handling |
|---|---|
| Specialist/department-style manifest unreachable | Fall back to Classic-only, with a banner: "Couldn't load specialist picks right now — showing standard models. You can switch later in Settings." |
| No tier fits available VRAM at all (e.g. < 4 GB) | Show a single "Minimal" fallback tier (smallest available model per category, CPU-only) rather than blocking onboarding; flag clearly that performance will be limited. |
| A specific model's GGUF URL is dead | Caught at download time in Step 5, not here. |

---

### Step 5 — Confirmation & Configuration

#### A. UX Narrative

Single summary screen, no further edits possible except "Go back" to a specific earlier step (each summary line is tappable and jumps back to that step). Primary button: "Set up LastHarness." Once tapped, the screen transitions into a progress view with named phases (Writing configuration → Seeding memory → Downloading departments → Downloading models → Done), each with its own progress bar. The user cannot navigate away mid-setup without an explicit "Cancel setup" confirmation. Model optimization is no longer an automatic phase in this sequence — conservative-safe settings are applied instantly as part of the model-download phase, and the full sweep is offered separately as an opt-in on the Step 6 completion screen (see Step 6).

#### B. Data Contract

**Summary content:**
- Inference path: provider name, or "Local AI on this device"
- Personal memory: "Enabled (N fields filled)" or "Skipped"
- Departments: list + total size
- Model pack (Path C only): pack name + mode (Classic/Specialist/Manual) + total download size + estimated download time at 50 Mbps

**On confirm, in order:**

1. Write final `lastharness.config.yaml` — merges the partial configs produced by Steps 1, 3, and (if Path C) 4/6.
2. Seed Memory department from Step 2's staged data (sanitization gate applied here, per Step 2's spec).
3. Download selected department packages; verify each against manifest checksum after download.
4. (Path C) Download selected model GGUFs; verify each against `model_selection.json`'s `sha256`.
5. (Path C) Trigger llama-bench-maxx (Step 6).
6. Launch main LastHarness UI.

**Final `lastharness.config.yaml` shape (illustrative, merged):**

```yaml
schema_version: 1
inference:
  mode: local
  base_url: http://127.0.0.1:8899   # set once Step 6 completes
departments:
  active: [kernel, information, security, development, research]
onboarding:
  completed_at: 2026-08-04T10:41:07Z
  personal_memory_seeded: true
  model_pack_mode: specialist
```

#### C. Failure Handling

| Failure | Handling |
|---|---|
| Download interrupted mid-file (any package or model) | Resume from last received byte offset via HTTP Range requests on retry — never restart a large model download from zero. |
| Checksum mismatch after full download | Delete the file, retry the full download once automatically; if it fails checksum a second time, surface: "One of the downloads didn't come through correctly. Try again?" with a manual retry button. |
| Disk fills up mid-download | Pause all downloads immediately. Modal: "You're out of space. Free up about [X GB] and we'll pick up where we left off." Poll free space in the background; auto-resume once sufficient space is detected, or let the user tap "Try again." |
| Department package download 404s | Treat as if GitHub were unreachable for that one entry: skip it, note it as "unavailable — try again from Settings later," continue with the rest. Do not fail the entire setup over one optional department. |
| User cancels mid-setup | Confirm ("Setup isn't finished — quitting now means LastHarness won't be usable yet. Cancel anyway?"). On confirm, leave partial `lastharness.config.yaml` marked `onboarding.completed_at: null` so the app knows to resume onboarding (not restart it from Step 0) next launch. |

---

### Step 6 — Model Optimization *(Path C only, offered after all model downloads complete)*

#### A. UX Narrative

The moment downloads finish, LastHarness applies conservative-safe settings to every downloaded model automatically — no sweep, no wait, no user action required. The setup progress view (Step 5) moves straight to "Done," and the user can enter the app immediately.

On the same completion screen, an **"Optimize now"** button is shown alongside the "Continue" button — visible, not buried in Settings, but clearly optional. Subtext: "Your models are ready to use. Optimizing can make them noticeably faster, but takes time and uses your CPU/GPU while it runs."

Tapping "Optimize now" surfaces a confirmation warning before anything starts:

> **"This can take a while — from several minutes to over an hour depending on your device — and LastHarness will use extra CPU/GPU the whole time. You can cancel at any point and keep your current (working) settings."**

Buttons: "Never mind" / "Start optimizing." Only "Start optimizing" begins the llama-bench-maxx sweep described in Section B below. If declined, the user proceeds into the app on the conservative defaults, with the same "Optimize now" entry point available later from Settings — unchanged from the existing Settings behavior.

Once running, the sweep view itself is unchanged from the original design: current model name, current phase (1–4, labeled in plain language: "Finding the fastest GPU setting," "Tuning performance," "Checking speed boost support," "Finding maximum memory size"), the best result found so far for that model ("Currently: 34 tokens/sec"), and an overall time-remaining estimate derived from sweep progress rate. Per-model "Skip this model" control, and a global "Cancel remaining optimization" control — both explained inline ("Skipping means this model will still work, just not as fast as it could be").

#### B. Data Contract — Sweep Specification

Runs once per downloaded model, in category order (smart → mid → fast → coding → vision → thinking → tool-calling).

> **Measurement-tool correction — 2026-09-07 (owner decision).** The sweep measures with **`llama-benchy`** (eugr/llama-benchy, MIT-licensed Python tool that benchmarks OpenAI-compatible endpoints), **not `llama-bench`** — the earlier text named llama-bench; recorded here rather than rewritten silently. Why: `llama-bench` drives the llama.cpp C++ engine directly, and those numbers are not representative of the path LastHarness actually serves on — the kernel and departments hit `llama-server` over `/v1/chat/completions` via `llama-swap` (§16.1). Engine-direct readings inflate what the owner will actually experience (no HTTP/streaming/chat-template overhead), and reusing the same prompt across sweep runs hits llama-server's prefix cache, inflating prompt-processing figures further. `llama-benchy` benchmarks the real endpoint with realistic book-text prompts (Project Gutenberg) and an HF tokenizer for accurate counts, discarded warmup runs, latency-subtracted prompt processing (`--latency-mode generation`), cache-avoidance noise (`--no-cache`), and correct multi-token-prediction chunk handling — which Phase 3 needs anyway. The sweep keeps its name (`llama-bench-maxx`); only the measurement tool changes. Mechanically: the driver launches `llama-server` per candidate flag set, points `llama-benchy` at it, and tears it down; OOM detection therefore moves to the server process (model-load failure in its stderr) plus benchy request failure.

**Phase 1 — GPU layer sweep**

- Sweep `--n-gpu-layers` from 0 to the model's declared layer count (read from GGUF metadata `llm.block_count`), step size 4 (step size 2 for models under 4B parameters, where layer counts are small enough that step-4 could skip the optimum entirely).
- At each step: launch `llama-server` with the candidate flags and run `llama-benchy --tg 128 --runs 3 --warmup-runs 1 --no-cache --latency-mode generation --format json` against it. 128 is large enough to get a stable tokens/sec reading past initial-load noise, small enough to keep the sweep from dominating total onboarding time (a full sweep is already O(dozens) of runs); the discarded warmup run covers the rest of the load noise.
- OOM detection: `llama-server` fails to start or load the model (non-zero exit, or presence of `CUDA out of memory` / `Metal error: -1` / `out of memory` in its stderr — document all three per-platform patterns explicitly in the implementation), or `llama-benchy` cannot complete requests against the started server.
- On OOM: step back one increment from the failing value, mark that as the ceiling, stop sweeping upward for this model.

**Phase 2 — Parameter sweep at winning `n_gpu_layers`**

| Parameter | Sweep values | Notes |
|---|---|---|
| `--threads` | 1 → physical core count, step 2 | Only matters materially for CPU-resident layers; still swept even at full GPU offload since some ops remain CPU-bound. |
| `--n-batch` | 128, 256, 512, 1024 | Covers the practical range llama.cpp users report meaningful throughput differences across, without an unbounded sweep. |
| `--n-ubatch` | 64, 128, 256 | Must be ≤ `--n-batch` for the paired combination being tested; invalid pairs are skipped rather than run. |
| `--cache-type-k` / `--cache-type-v` | f16, q8_0, q4_0 | Only swept if GGUF metadata indicates the architecture supports sub-f16 KV cache (presence of `llm.attention.head_count_kv` plus a minimum llama.cpp version check) — sweeping this on an unsupported architecture just produces guaranteed failures. |
| `--flash-attn` | on / off | Binary toggle. |

Total combinations and estimated wall-clock time for this phase must be documented and shown to the user as part of the time-remaining estimate; on a mid-end device this phase is expected to be the single largest time cost of Step 6.

**Phase 3 — MTP (Multi-Token Prediction) sweep, conditional**

- Detect MTP/speculative-decoding support via GGUF metadata (exact key to verify against current llama.cpp source at implementation time — flagged as an open item, likely under an `llm.*` or `tokenizer.*` namespace key indicating a draft-model relationship).
- If present: sweep draft token counts [1, 2, 4] at the Phase 2 winning parameter set.
- Record both tokens/sec *and* accepted-token rate per draft count — a higher draft count with a low acceptance rate is a net loss and must not be selected purely on raw tokens/sec. (`llama-benchy`'s realistic book-text prompts and MTP-chunk-aware counting are what make the accepted-token rate meaningful here; random-token prompts would over-accept and inflate the reading.)

**Phase 4 — Maximum context size discovery**

- Binary search `--ctx-size` between a hard floor of 4096 (below this, the model isn't usefully conversational for most tasks, so it's not worth searching further down) and the model's declared max context (`llm.context_length` from GGUF metadata).
- Narrow until the search range is ≤ 512 tokens wide; take the lower of the two final candidates as the safe max (never round up into untested territory).
- Before running each candidate, estimate KV cache memory analytically (context size × layer count × head dimensions × cache dtype size) to skip candidates that are obviously going to OOM, saving sweep time.
- If even the 4096 floor OOMs: flag the model as broken for this device, skip it entirely, surface a clear warning ("This model needs more memory than your device has — we've left it out of your setup").

#### C. Output — Per-Model llama-swap Config Entry

```yaml
# llama-swap.yaml (one entry per successfully benchmarked model)
models:
  coding:
    cmd: >
      llama-server
      -m /models/kat-coder-v2.5-Q5_K_M.gguf
      --n-gpu-layers 32
      --threads 8
      --n-batch 512
      --n-ubatch 128
      --cache-type-k q8_0
      --cache-type-v q8_0
      --flash-attn
      --ctx-size 24576
      --port ${PORT}
    # MTP omitted here if not supported/not beneficial for this model
```

Benchmark summary metadata (tokens/sec achieved, sweep date, device tier) is stored **alongside**, not inside, the llama-swap config — a separate `benchmark_results.json` consumed by the Settings UI for later display, so re-running llama-bench-maxx after a hardware change doesn't require parsing launch flags back into human-readable numbers.

```json
{
  "coding": {
    "model": "kat-coder-v2.5",
    "tokens_per_sec": 34.2,
    "max_ctx": 24576,
    "mtp_enabled": false,
    "benchmarked_at": "2026-08-04T10:52:30Z",
    "device_tier": "mid"
  }
}
```

#### D. UI / Failure Handling for This Step

| Situation | Handling |
|---|---|
| User declines "Optimize now" on the completion screen | Every model keeps the conservative defaults applied automatically at download completion (Section A above) — same defaults as the per-model-skip row below, just applied to all models rather than one. No further action needed; "Optimize now" remains available in Settings indefinitely. |
| User skips a model (mid-sweep) | Conservative defaults applied for that model: `--n-gpu-layers` set to a value estimated to fit safely within 80% of detected VRAM budget (no sweep), `--ctx-size` set to 8192 flat, no KV quantization, no flash-attn, no MTP. Model is usable, just unoptimized; Settings later offers "Optimize now." |
| User cancels remaining benchmark | All models already fully swept keep their optimized config. All not-yet-started models get the same conservative defaults as the per-model skip case above. |
| Full sweep takes far longer than estimated (e.g. thermal throttling extends runtime) | Time-remaining estimate re-derives continuously from actual completed-sweep rate, not a static initial estimate — should self-correct rather than stay wrong. |
| A model OOMs even at the absolute minimum settings tested | Flagged as broken for this device (see Phase 4); excluded from the final `llama-swap.yaml`; category falls back to the next-smaller model in the same pack tier if one was bundled, otherwise the category is left unconfigured with a clear "not available on this device" state in Settings. |

**On completion (either path):** write final `llama-swap.yaml`, set `inference.base_url` in `lastharness.config.yaml` to the local llama-swap endpoint, mark `onboarding.completed_at`, and hand off to the main LastHarness UI. If the user declined optimization, this happens immediately after conservative defaults are applied at download completion. If optimization is run later from Settings, it updates `llama-swap.yaml` and `benchmark_results.json` in place without re-touching onboarding state.

---

*End of onboarding specification.*

---

*View Source · Edit this page · Report Issue*

© 2026 LastHarness Framework. Open source under MIT License.
