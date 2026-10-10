# LastHarness — Department Blueprints (Feature-Exploration Wave 1 synthesis)

> Distilled from 40 feature dossiers (`10-feature-dossiers/wave1/`) covering all 33 adopt-tier entries plus the 7 strongest slot-notes. Method: README/docs-driven inventories, every claim source-marked; verdicts against the locked invariants (§16.1 self-hosted boundary · §16.5 ephemeral workers · §4.4 kernel mediation · §7.1 egress default-deny). Companion to the shallow catalog in `09-feature-extraction.md`; this file is the *architectural* read.

---

## 1. Recurring-feature table — features in 3+ tools with no LastHarness home yet

Each row is a **department-gap candidate**: recurrence across independent tools is field evidence that LastHarness's spec lacks a named home for the capability.

| # | Recurring feature | Tools exhibiting it (dossier evidence) | Recurrence | Current LastHarness coverage |
|---|---|---|---|---|
| R1 | **Human-approval gate before irreversible / side-effectful actions** | cline (default-on per-action gates), browser-skills (pause before job apply/RSVP), support-skills (confirm-before-external-action as mandatory last step), claude-code (plan gate blocks edits until approved), cursor (Agent Review), open-kritt (validate-before-run workflow) | 6 | §13.3 covers only *irreversible* actions + owner biometric; no general approval-gate mechanism in the dispatch contract |
| R2 | **Plan-as-editable-artifact, propose-then-execute two-phase** | cline Plan/Act split, cursor Plan Mode, claude-code plan mode + Ctrl+G editable plan, deep-research-skills outline.yaml phase, open-kritt validate-before-run | 5 | §4.6 sketches plan→dispatch→verify→replan but never specifies a plan artifact the owner can edit/gate |
| R3 | **Compaction-time structured manifests & resumable snapshots** | token-goat PreCompact MUST_PRESERVE manifest, context-mode FTS5/BM25 index-not-dump snapshots, ecc session-lifecycle memory hooks, deerflow session reuse + SSE gap events, scrapling crawl pause/resume | 5 | §7.1 defines a compaction *threshold* but no compaction protocol/artifact/resume contract |
| R4 | **Trust/staleness/provenance-gated memory promotion** | token-goat-mem (anchor predicates, pending→human-review→promote, never auto-promote), tencentdb L0→L3 layered distillation, qiaomu-seo (observed/inferred/missing evidence grading), deep-research `[uncertain]` convention, scientific-agent-skills provenance-pinned installs | 5 | §6.3 has freshness cutoffs; adopted staleness anchors (#5) cover validation — but the *promotion-to-review workflow* is unspecified |
| R5 | **Skill-pack supply chain: envelope standard + lazy loading + security scanning** | SKILL.md folder+frontmatter standard ×5 packs (cybersecurity, codex, claude-plugins, support, emil), deerflow lazy package-boundary loading, google/skills selective install, K-Dense weekly security scans + version pinning | 8 | §14.5 `skills` category exists; intake lifecycle (provenance → scan → pin → register) does not — flagged twice before, now evidenced at scale |
| R6 | **Structured partial-failure channels & typed errors** | tavily `failed_results[]{url,error}`, anydoc typed error taxonomy w/ resource caps, t-search degenerate-loop force-terminators + error-as-feedback | 3 | §16.6 Output 1 has terminal status only; no shared error-shape standard for tool results |
| R7 | **Owner visibility surfaces over live work** | conductor (review board: worktree diffs side-by-side, Checks-tab aggregate gate), langfuse trace UI, windsurf Cascade dashboards, prior-wave #77 Devin digests | 4 | Notification queue + decision log exist; **no owner console section anywhere in spec** — third time this gap has surfaced |
| R8 | **Delegation-tree budgets w/ cycle detection** | pi-rlm maxDepth/maxNodes/maxBranching + lineage cycle detection, omp worktree fan-out caps, t-search round/budget hard-lock | 3 | §4.6 aggregate-limit rule exists; per-node recursion budgets and runaway containment do not |
| R9 | **Scoped tool-profiles shrinking down the delegation chain** | pi-rlm read-only toolsProfile for delegates, claude-code dontAsk allowlist inheritance, needle-2 top-5 tool retrieval as dynamic allow-list | 3 | §16.6 sets allow-lists at dispatch; no rule that children inherit a *subset* — deny-by-default currently stops one level deep |
| R10 | **Append-only event streams as audit substrate** | pi-rlm events.jsonl, claude-code protected transcripts, awesome-claude-plugins asqav hash-receipt session logs, aider auto-commit-per-edit trail | 4 | Decision log exists (§12); worker-level event streams feeding it are unspecified |

**Validated exclusion class (recurring ❌):** standing schedulers/inboxes (cline schedules+connectors, holaOS automations/channels, token-goat worker daemon), default-on cloud/telemetry (OmO PostHog, context-mode Insight, tencentdb gateway), third-party/cloud execution (cursor cloud agents, kritt root containers, Composio gateway, windsurf Netlify deploys, Devin Outposts control-plane). Every dossier independently re-derived the same litmus — the invariants are load-bearing and market-standard.

---

## 2. Department enrichment map — where ⚙ features slot into the six existing departments

### Development (§14.2)
- aider post-edit lint/test auto-fix loop → QA-engineer dispatch tail
- ponytail 7-rung YAGNI ladder + security carve-outs → per-dispatch instruction pack (already adopt)
- karpathy surgical-changes line-traceability policy → backend/frontend role instructions
- ecc role-scoped subagent delegation contracts → manager config schema
- oh-my-pi LSP diagnostics evidence channel + atomic willRenameFiles handling → refine-loop evidence (§10.2)
- cline checkpoints/undo per mutating op → worktree discipline (§16.6)
- cursor Rules/Hooks declarative stack → role config-slice conventions
- oh-my-openagent hashline anchored edits + category→model routing → strict-anchor contract (§16.6, already harvested)

### Research (§14.3)
- tavily cleaned-extract response shape → benchmark target for SearXNG wrapping
- google-skills SLO/log query-wizard pattern → web_researcher query scaffolding
- scientific-agent-skills deterministic DB-lookup skill template → literature/data roles
- anydoc canonical-markdown ingestion (adopted) → Document Store pipeline
- deep-research two-phase outline fan-out (adopted) → manager flow

### Intelligence (§14.4)
- needle-2 confidence-gate escalation routing (adopted) → cost/escalation thresholds
- local.ai setup-level benchmark dataset (adopted) → Step 4 pre-measured tables
- oh-my-openagent category→model automatic delegation → alias-router generalization
- windsurf adaptive model router → corroboration of tier-per-task routing
- searxng engine-suspension ladder (captcha/429 backoff) → search-provider health policy
- langfuse cost/latency telemetry fields → quality_monitor sampling schema

### Information (§14.5)
- token-goat-mem trust tiers + review-promote gate (adopted) → memory write-gate upgrade
- tencentdb L0→L3 distillation pyramid + hybrid BM25/vector RRF on local sqlite-vec → Knowledge Base layout
- context-mode FTS5 index-not-dump resume snapshots → compaction protocol (R3)
- ecc session-lifecycle memory hooks → session archivist triggers
- deerflow lazy skill loading w/ package boundaries (adopted) → skills registry retrieval
- windsurf `.devinignore` exclusions → gated repo-index access shape

### Automation (§14.6)
- scrapling AutoThrottle + robots_txt_obey + pause/resume crawl state → scheduled-job policy template
- windsurf auto-worktrees per parallel task → §16.1 confirmation at product scale
- support-skills P0–P3×SLA triage taxonomy → alert_dispatcher severity scheme
- ❌ cline schedules/connectors, holaOS automations/channels — standing processes, excluded by invariant

### Security (§14.7)
- open-kritt validator-reproduced PoC pipeline (adopted) → threat_detector workflow
- spiderfoot YAML correlation rules (declarative evidence→risk matching) → incident_log rule schema
- anthropic-cybersecurity framework-mapped packs (ATT&CK/CSF/D3FEND frontmatter, adopted) → Sec skill content
- yap refuse-feature-over-network-fallback doctrine → egress posture corroboration
- K-Dense fail-closed analysis posture + weekly skill scans → supply-chain hygiene (R5)
- windsurf terminal allow/deny + Security Profiles + AI Guardrails → validates §7.1/§13.4 defaults

---

## 3. Top-5 new-department proposals — ranked by recurrence × invariant-fit

### 1. Owner Console & Approval Gate *(kernel-tier surface)*
- **Why #1:** strongest recurrence (R1×6 + R2×5 + R7×4 = three gaps, one surface), zero invariant friction — it *strengthens* §13.3/§16.9 rather than bending anything.
- **Authority scope:** render kernel state read-only (routing queues, active worktrees, decision log); host approval gates for staged irreversible actions; accept/edit plan artifacts before dispatch fan-out.
- **Tool evidence:** cline, browser-skills, support-skills, claude-code, cursor, conductor, open-kritt, deep-research-skills.
- **Spec work:** new **§16.10 "Owner Surfaces"** — approval-gate contract (what stages, what the owner sees, typed confirmations), plan-artifact format, review-board data feeds.

### 2. Skills Registry & Supply Chain *(Information dept expansion)*
- **Why:** R5×8 is the single most-recurred cluster; sits wholly inside existing §14.5 authority (skills category already exists).
- **Authority scope:** skill envelope standard (SKILL.md + scripts/references/assets), intake lifecycle (provenance → sandbox scan → version pin → register), lazy-loading retrieval protocol.
- **Tool evidence:** the five SKILL.md-standard packs, K-Dense scans/pinning, deerflow lazy boundaries, google selective install.
- **Spec work:** **§14.5 addendum "Skill Intake Lifecycle"** absorbing design-notes from 09 (#21/#25/#26/#27/#28/#35/#36/#39/#41/#68/#98/#108).

### 3. Context Economy Service *(Intelligence dept expansion)*
- **Why:** R3×5 plus already-specced token levers; turns §7.1's threshold into an actual protocol.
- **Authority scope:** compaction artifact/resume contract, output normalization (compress/shrink before context entry), bounded-retrieval worker family.
- **Tool evidence:** token-goat manifest, context-mode FTS5 snapshots, ecc hooks, deerflow sessions, scrapling resumable crawls, tavily response shape.
- **Spec work:** **§7.1 addendum "Compaction Protocol"** + §16.6 output-normalizer step.

### 4. Verification & Evidence Service *(Security dept expansion)*
- **Why:** R6×3 + R10×4 + evidence-honesty conventions (R4 adjacency); completion-gates are the concrete definition §11.3's "verification suite" still lacks.
- **Authority scope:** worker completion-gates (schema validators, grammar-constrained outputs, test-suite CI), typed failure-channel standard, event-stream substrate feeding §12.
- **Tool evidence:** needle grammar constraints, deep-research validate_json gate, K-Dense CI, tavily failed_results, anydoc typed errors, pi-rlm events.jsonl, asqav hash receipts.
- **Spec work:** **§16.6 addendum "Result schemas v2"** (error shapes + event stream) + **§11.1 note** defining verification-suite artifacts.

### 5. Edge & Device Ops *(Intelligence-adjacent pack family)*
- **Why:** coherent sub-Low-tier cluster (needle/yap/local.ai/RF-DETR) but weakest recurrence — rank 5 on purpose.
- **Authority scope:** hardware tier below Low (MCU/wearable/budget-phone), escalation thresholds to cloud above confidence gates, OS-native-capability preference doctrine.
- **Tool evidence:** needle-2 (28MB RAM tool-caller), yap (zero-model speech), local.ai energy benchmarks, RF-DETR edge detection.
- **Spec work:** **§17.3 extension "Below-Low tier"** + §14.4 escalation-threshold policy.

*Deliberately not proposed:* a fifth "Team Memory" department (withdrawn precedent, §14.5 addendum 2026-08-11) — every wave-1 memory feature mapped into Information without residue.


---

# Wave 2 findings (conflict teardowns + harvest/notes depth-pass)

> Source: 36 additional dossiers in [`10-feature-dossiers/wave2/`](archive/10-feature-dossiers/wave2/) — all 12 Conflict-tier entries (full teardowns with compliant re-implementation sketches) plus 24 note/harvest tools. Skeptic pass applied (1 BLOCKER fixed: deep-agents harness-as-runtime downgraded to §16.2 pattern-donor). ~394 further features inventoried, source-marked. R1–R10 above are unchanged; this section extends them.

## Strengthened existing rows
- **R7 Owner Console (strongest proposal, now near-certain):** t3-code contributes a full remote-auth ladder (one-time pairing token → device session → revocation ledger; tailnet-first transport) + inline remote approval queues; opentag contributes `confirm_write` resumable-interrupt approval cards; herdr's teardown yields the working/blocked/idle status triad rendered free from §16.6 Output statuses. §16.10 now has transport, auth, approval UX, and status vocabulary all evidenced.
- **R10 Audit streams:** two conflict-tier tools independently reinvented §12 (buzz hash-chained event log; cloudflare-computer SQLite-authoritative state) — the decision log is validated as ahead-of-market rather than merely adequate.
- **R5 Skills supply chain:** oh-my-hermes' 36-skill layer + qiaomu-seo's stale-source registry (33 dated sources, strict-stale auto-block) add the freshness-governance piece to the intake lifecycle.

## New recurring gaps (R11+)
| # | Gap | Evidence | Landing |
|---|---|---|---|
| R11 | **Kernel-composed handoff packet as named artifact** | ai-memory typed handoffs + munder-difflin routing converge | §16.6 dispatch inputs |
| R12 | **Evidence-ledger append-policy for the decision log** | comp-crm write/suggest/prefer-blank thresholds; exa `sources_reviewed` counters; qiaomu tri-label grading | §12 / Verification Service proposal #4 |
| R13 | **Incident-rule DSL** (declarative match→aggregate→analyze→typed-risk compiling to SQL over event tables) | spiderfoot correlation engine | §14.7 incident_log schema |
| R14 | **Knowledge-freshness governance** at skill/knowledge intake | qiaomu-seo dated-source registry + strict-stale validator | §14.5 intake lifecycle |
| R15 | **Hardware-capability probe → generated compat matrix** as pack-selection primitive | lemonade `backends` probe; local.ai setup benchmarks | §17.3 Below-Low tier (feeds proposal #5) |
| R16 | **Single-artifact packaging contract** for dept tool packs incl. two-tier backup semantics | pocketbase one-binary shape | installer/§16.1 packaging reference |

## Proposal amendments
- **#1 Owner Console**: absorb remote-auth spec (pairing-token sessions + revocation + inline approval queue) and the status triad — t3-code is the reference implementation class.
- **#4 Verification & Evidence Service**: absorb comp-crm's ledger pricing (strong evidence → auto-write; weak → human-settled suggestion; blank beats confidently-wrong) as the §12 append-policy; exa/qiaomu conventions as field schemas.
- **#5 Edge tier**: strengthened by lemonade's machine-capability probe — make generated compat matrices a Step-0/Step-4 primitive.

## New department cases (parked)
- **Meeting-Ops**: Meetily proves fully invariant-clean local meeting capture↔summary; park until owner demand exists.
- **Media Production**: ViMax + Stitch prove demand; hard-blocked on local image/video generators — opens only when the §16.4 fast-follow triggers fire.

## Surprises & data corrections
- **Flowise repo ARCHIVED** upstream; license shows NOASSERTION vs Apache-2.0 badge — pattern-donor only, adoption closed.
- **ViMax stars corrected** to 12,084 (registry said ~9.9k).
- **oh-my-hermes license conflict**: README badge says MIT, GitHub API reports none — resolve provenance before any code reuse.
- **deepseek-harness admission gate** ("model-visible means logged" rewrite-or-reject pre-step) is a novel §16.6 input-gate mechanism worth a design note even though the harness itself stays excluded.


---

# Wave 3 findings (full-coverage confirmation pass)

> Source: **56** dossiers in [`10-feature-dossiers/wave3/`](archive/10-feature-dossiers/wave3/) — the entire remaining registry (skip-tier confirmations + note-tier depth), plus the #132–136 drain batch. **No new recurring-gap rows emerge**: every pattern found strengthens an existing row or proposal rather than opening a new one.
>
> *Count corrected 2026-08-31: this said 51, which was right before the drain batch appended 5.*
>
> **Coverage claim qualified — 2026-08-31.** The original text asserted "registry coverage is now COMPLETE — all 130 active entries have wave-level dossiers." That overstates it: **three entries have no dedicated dossier** — #116 Capptivo (skip), #118 Roboflow RF-DETR (note), #127 holaOS (conflict). #127 is covered indirectly via `wave4/qwenpaw.md` (the "Odysseus/holaOS competitive cluster"); #116 and #118 appear only in their registry rows (`07-media-apps.md`, `04-local-inference.md`). None is adopt-tier, so no extraction was lost — but the word "COMPLETE" was not earned.

## Strengthenings (by existing row/proposal)
- **§7.1 egress defaults:** ChatGPT Codex ships per-command network-off with allowlist-first proxy vocabulary + deny-wins and DNS-rebinding checks — the strongest prior art in the registry; adopt vocabulary verbatim into the gateway spec.
- **Proposal #2 Skills intake:** pin/quarantine-before-trust independently evidenced three more times (agent-skills-flow vendir lock, gstack domain-skill quarantine, elevenlabs trigger-eval-before-register).
- **Proposal #4 Verification & Evidence:** fail-closed generation gates that refuse compute before spend (img2threejs `BLOCKED`-with-metrics pre-codegen gate; Remotion schema validation); verifiable text artifacts over binaries (Remotion/img2threejs/HermesOffice convergence); MiMo outcome-tagged result-log as §12.4 field evidence.
- **R7 Owner Console:** grok-build ACP runtime/client split + superset remote views + wekan board render — third/fourth/fifth corroborations.
- **§16.6 settled consensus:** worktree-per-dispatch + review-before-merge confirmed independently 3× in this wave alone (grok-build/superset/muse-code); fresh-context-per-step with verified-state carryover recurs as a quality lever (longhorizon, prime-agent).

## Registry integrity corrections (logged to 99-corrections.md)
#70 LocalStudio repo found (sybil-solutions/local-studio) · #71 KittyLitter repo found (0xSero/litter via dnakov redirect) · #38 Composio Open Agents dissolved into stale fork of #37 (merge candidate) · #115 Handle + #129 AgentReach delist recommended · #53 star-inflation flag RESOLVED (~235k genuine) · license-null pair flagged (#8, #98).

## Skip-tier verdict
Every recorded skip reason survived primary-source contact — zero dispositions overturned across 26 skip-tier confirmations. The flat-table skips were right for structural reasons (§16.2 SDK lock, §16.1 boundary, single-owner scale), and primary sources keep validating them post-hoc.

---

## Wave 3 addendum: Impeccable (#132, added 2026-08-25)

Late registration from user's live saves — a design-language skill already installed in the environment. Full dossier at `10-feature-dossiers/wave3/impeccable.md`. Key findings that touch existing rows/proposals:

- **Strengthens R5 (skills intake)**: Impeccable's DESIGN.md is Google-Stitch-format-compatible with adopted #25/#26 — three tools now converge on one portable design-context format.
- **Strengthens Evidence Service proposal #4**: its deterministic 59-rule detector (zero LLM, CI exit codes) is exactly the completion-gate pattern proposal #4 specifies — strongest single prior art in the registry.
- **New mechanism worth recording**: the **Worlds challenger deck** — 177 human-reviewed complete graphic systems dealt against the model's own derived direction, winner takes the build committed. This is an anti-convergence mechanism mirroring §16.9's asymmetric-evidence auditing but applied to *aesthetic selection* rather than verification. Recorded as 🧩 mechanism-only under Evolution System §10.3.


---

# Wave 5 findings (trending registrations)

> Source: 10 dossiers in [`10-feature-dossiers/wave4/`](archive/10-feature-dossiers/wave4/). No new gap rows — this wave's value is *prior-art depth* on existing rows and proposals.

## Cross-reference amendments
- **Proposal #1 Owner Console** — **#137 Hermes Studio** is the strongest single prior art found to date: a shipped web dashboard rendering another agent's sessions, scheduled jobs, and multi-platform chat with frozen run snapshots and edge-decision evidence playback. It demonstrates the console as a product category over a foreign runtime — exactly §16.10.1's shape. **#145 cmux** adds the terminal-side render (per-session sidebar, attention triage) as R7 evidence.
- **R11 Handoff packets** — **#138 LoopX** is the strongest prior art yet: typed handoff artifacts (claims/leases/task boundaries/typed continuation), durable goals surviving provider changes across Codex/Claude Code, quota-gated ticks (spend appended only after validated writeback), and audited safe-fallback structurally barred from bypassing gates. Its governed-handoff artifact set should be read alongside the adopted #59 save-gated rounds when drafting the §16.6 input schema.
- **Freshness completes both halves** — adopted #5 staleness anchors cover write-time anchoring; **#139 CodeGraph** contributes read-time enforcement (per-file staleness banner, connect-time hash catch-up, never answer stale silently). Together they specify §6.3 freshness end-to-end.
- **Provenance tagging confirmed cross-tool**: codegraph `synthesizedBy` edges, loopx evidence-strength labels, ruflo signed witnesses — trust-tier metadata in §6.3 is market-standard, not bespoke.
- **Standing-process conflict cluster re-confirmed 4× in one wave** (astrbot channels, qwenpaw heartbeats, ruflo queen/workers/federation, qwen-code daemon/IM bots) — the litmus keeps settling these without spec changes.

## License-integrity notes (logged in 99-corrections.md)
#137 resolves to **BSL-1.1** (source-available — pattern-level prior art only, no code reuse); #145 README claims GPL-3.0-or-later while GitHub API reports NOASSERTION — null-license flag stands pending LICENSE-file inspection.
