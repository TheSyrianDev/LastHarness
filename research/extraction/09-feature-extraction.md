# 09 · Feature Extraction Catalog — mechanisms worth taking, products left behind

> Method: six parallel lanes re-examined all 108 active registry entries against docs/archive/lastharness-v1-spec.md's locked architecture. Question asked per tool: *"what MECHANISM would LastHarness steal?"* — the way Prime Agent's RLM decomposition is worth more than Prime Agent itself. Every disposition names its spec slot or the invariant it would violate. Skeptic pass consolidated independent proposals of the same mechanism. Run journal: `archive/runs/ulw-research/20260822-143607/`.

**Dispositions:** **Adopt** (39) fits invariants, concrete slot named · **Note** (58) valuable principle, wrong shape/time for V1 — record like the spec's harvest addenda · **Conflict** (17) violates a locked invariant · **Skip** (32) nothing beyond what LastHarness already specs (confirmation evidence). Total **146** ledger rows.

> **Count corrected — 2026-08-31.** This header previously read 29 / 40 / 12 / 27 = 108. Those were the #1–108 pass frozen at the time it ran; waves 5 and 6 appended 39 more rows without updating them. Verified by parsing the appendix ledger: 39 adopt / 58 note / 17 conflict / 32 skip = 146. See `14-research-rescan-2026-08-31.md` §B and the corrections ledger.

---

## A. Adopt — 29 mechanisms, grouped by landing zone

### A1. Dispatch-contract hardenings (§16.6 / §16.5)
| Sources | Feature | Mechanism | Why it lands |
|---|---|---|---|
| 2 · 29 · 44 | SDK lifecycle-hook gates | Pre/post tool-call hooks host every deterministic gate (allow-lists, sanitization, clamps) at one boundary | Turns deny-by-default into code-enforced policy; three tools converged on the same substrate |
| 4 | Tool-result injection scan | Deterministic pattern scan of fetched/tool content before context entry | Research workers ingest untrusted web content today; complements §12.6's write-side gate |
| 11 | Per-call output clamp | Hooks bound each tool call's returned bytes/summary; raw output stays task-scoped | Distinct lever from whole-context compaction threshold (§7.1) |
| 54 · 92 | Scope-keyed instruction rules | Glob/path-keyed rule files activate per task scope above global role instructions | Precise steering without monolithic prompts; rides §16.6 input 6 |
| 91 | Commit-per-changeset discipline | Auto-commit accepted edit sets inside the worktree during the run | Strengthens diff-as-truth (§16.6); free rollback checkpoints |
| 59 | Save-gated retrieval rounds | Fresh-context rounds; only explicitly-saved chunks carry forward | Concrete instantiation of the harvested RLM "managed slots" principle on legal MCP surfaces |

### A2. Memory & knowledge lifecycle (§6.3 / §14.5)
| Sources | Feature | Mechanism | Why it lands |
|---|---|---|---|
| 5 | Staleness anchors + trust tiers | Facts anchored to file/git revisions; recall re-validates anchors, filters stale | Makes code_index rot machine-checkable; upgrades §6.3 freshness cutoffs |
| 24 | Interaction-distillation curation job | Scheduled job converts Session Archive/docs into semantic assets | Fills the curation gap as an Automation job over existing stores |
| 104 | Canonical-markdown ingestion | Convert PDFs/docs to structured markdown once; index uniformly | Uniform retrieval/embedding regardless of source |

### A3. Skills supply chain (§14.5 skills / §5.4 / §16.8 Step 2)
| Sources | Feature | Mechanism | Why it lands |
|---|---|---|---|
| 108 | Harness-validated skill intake | Imported skills pass automated sandbox checks before entering registry | Provenance-plus-harness gate; closes the intake gap five lanes flagged |
| 35 | Vetted catalog distribution | Checksummed manifest distribution of reviewed skills/MCP packages | Extends §5.4's department-manifest pattern to skills |
| 36 | Engine-agnostic skill corpus | Portable skill artifacts ingested regardless of originating harness | Seeds prove cross-harness portability of the skills category |
| 21 · 27 · 28 · 39 · 41 | Content packs | MITRE/NIST security, Karpathy pitfalls, motion-design, browser ops, support SOPs — loaded per-role at dispatch | Pure content through the registry; proves non-coding departments from packs |
| 25 | Brand-context injection | Project DESIGN.md injected into frontend-role dispatch excerpts | Mirrors the repo-map pre-step for brand/UI consistency |

### A4. Verification & security (§14.7 / §10.2 / §11.3)
| Sources | Feature | Mechanism | Why it lands |
|---|---|---|---|
| 73 | Validated-vulnerability pipeline | Discovery agents surface candidates; independent validators reproduce PoC evidence before reporting | Reproducibility gate matches stage-don't-execute culture |
| 93 | Shadow-git step checkpoints | Snapshot before each mutating op; failed verification rolls back to last checkpoint | Makes capped refinement loops (§10.2) cheap to iterate |
| 97 | Outline→fan-out gating | Approved outline artifact precedes parallel per-item children, then synthesis | Human-checkable intermediate before burning parallel budget |

### A5. Reference data & adopted-stack wiring (Appendix A / §14.3)
| Sources | Feature | Mechanism | Why it lands |
|---|---|---|---|
| 69 | Tier-reference benchmark dataset | Public models×hardware×energy figures feed pre-measured tokens/sec tables | Fills Step 4's pre-measured references before any live sweep |
| 90 | SearXNG JSON-API wiring | Expose SearXNG's JSON endpoint as WebSearch implementation behind role allow-list | Concretizes the already-made §16.4 decision; installer-provisioned like llama-swap |

---

## B. Design notes — record, don't build (like the spec's harvest addenda)

**Dispatch-flow family — "decompose → gate → fan-out."** Four lanes converged on one spine §4.6 sketches: scope-forcing interview before decomposition (1 GStack office-hours, 6 Pocock grill-me), plan artifact gated before execution (48 Claude Code plan checkpoint, 93 Cline plan/act), approved outline preceding parallel children (97), chunked child delegation for oversized inputs (107 pi-rlm). Record once as a §16.9 dispatch-flow family, not four tricks.

**Token/context levers beyond V1:** reversible compression with restore-on-demand (12 Headroom — lossy summarize risks discarding needed detail); budget-phase tool locking (59 T-search — narrows *tool access* as budget depletes, not context).

**Observability:** define the per-dispatch trace schema ONCE in §16.6 Output 3 (span-per-call telemetry: tool invocations, token counts, latencies — 64 LangFuse); dispatch-tree visualization over it deferred (23 VoltAgent).

**Evolution-proposal evidence policy:** permissible refinement evidence = machine-verifiable outcomes only — outcome-tagged trajectory mining (51 MiMo), bounded post-task reflection through sandbox verification (53 Hermes learning loop), population-based generational search deferred on cost (58 Catalyst ~$1000/workflow). Sharpens §10.2's verifier-gated loop and §12.4.

**Remote owner control plane (post-V1):** mobile/web/desktop monitor-and-approve clients riding §13.4 tunnels (50 T3, 71 KittyLitter); agent-client protocol split for future owner surfaces (49 Grok Build ACP). One protocol, specced once.

**Post-V1 messenger adapter shape:** external Slack/Teams threads map to owner-initiated kernel tasks; replies stream back (105 OpenTag adapter, 83 OpenClaw gateway) — defines the shape if §16.4's messenger clause ever reopens; persistent-warning pattern applies.

**Interface-shape notes (locked stack neighbors):** domain-partitioned search tools individually allow-listable (94 Exa); cleaned-extract response shape to benchmark SearXNG wrapping against (95 Tavily); multi-backend facade if modality services outgrow distinct clients (106 LocalAI); NPU serving path extends low-tier coverage (103 Lemonade — Step 0 scan ignores NPUs today); alias-routing generalized as an organizational pattern, not an inference accident (70 LocalStudio, 94, 95, 106).

**Authoring & artifacts:** visual graph editor compiling TO department.config.yaml, never runtime graphs (30 Flowise); declarative recipe schema for parameterized invokable tasks (86 Goose recipes → job_definitions); dept-aligned versioned pack layout (98 Flow monorepo); provenance-tiered sourcing preferring first-party packs (26); curated interlocking bundles (68 Oh-my-Hermes); agent-native document formats rendered losslessly to DOCX/PDF (75 HermesOffice); staged media-pipeline decomposition for the video-gen fast-follow (17 MoneyPrinterTurbo stages; 31 Remotion code-driven rendering).

**Owner visibility (candidate new section):** interim progress digests during long dispatches (77 Devin); parallel-dispatch review board listing live worktree diffs side-by-side (78 Conductor); read-only board render of routing-queue/notification states (15 Wekan pattern). The spec has queues and logs but no owner console — one section, not scattered notes.

**Smaller notes:** jargon glossary reused across installer/onboarding copy for the non-technical audience (8 Dictionary); credential decoupling — vaulted per-app creds injected at call time, for the gateway era (38 Composio); network-off-by-default per command inside the single boundary (47 Codex); canned mission templates making recurring intelligence work auditable (96 ads extractor); dispatch-tree replay/fork for recovery root-cause (66 LangGraph — V1 snapshot suffices); framework-mapped skill tagging (21's MITRE/NIST axis generalizes).

---

## C. Conflicts — mechanisms that violate locked invariants (do not adopt)

| Sources | Mechanism attempted | Violated invariant |
|---|---|---|
| 7 Sandcastle | Provider-abstracted sandboxing over Docker/Podman/Vercel backends | §16.1: third-party runtimes ruled out; remote execution excluded |
| 11 Context-Mode (session store half) | Worker-native durable SQLite event store surviving sessions | §16.5: no worker state outside MCP handle/worktree |
| 18 Munder-Difflin | Standing mutually-coordinated CLI-wrapping agents | §16.5 ephemeral dispatch; §16 wrapped-CLI exclusion |
| 19 AI Memory (sharing model) | Cross-agent shared wiki injected into every connected agent | §6.2/§4.4: memory is department-scoped, kernel-mediated |
| 32 Open Claude Cowork | Standing cowork shell + 500-integration breadth | §16.5 standing processes; §16.1 footprint |
| 33 Secure OpenClaw | Resident personal-assistant daemon on messaging channels | §16.5; §16.4 messengers out of V1 |
| 37 Vercel Open Agents (execution model) | Cloud background coding agents | §16.1 cloud-worker exclusion (2026-08-11) |
| 38 Composio (hosted half) | Agent processes on provider-managed cloud infra | §16.1 cloud-worker exclusion |
| 40 Composio connector fabric | Outbound-by-default account layer, managed auth | §13.4.2/§7.1 egress-gateway default-deny |
| 47 Codex (delegation half) | Tasks executed on OpenAI hosts | §16.1 cloud-worker exclusion |
| 53 Hermes (daemon half) | Persistent process answering Telegram/Discord directly | §16.5 standing process; unmediated channels |
| 61 Cloudflare Computer | Durable-Object VFS + remote sandboxed execution | §16.1 isolation boundary; §2.1 self-hosted |
| 62 OpenCodex | General provider-translation proxy; quota-evasion account pools | §16.1 no-proxy stance (Path B exception is user-supplied only); ToS-gray pools |
| 81 Herdr | Multiplexer keeping agent sessions alive indefinitely | §16.5; §14.6 scheduled re-check is the sanctioned path |
| 82 pi-black | Subscription-OAuth impersonation shim | §13 authentication chain / auditability (also 99-corrections flag) |
| 87 Buzz | Peer relay channels between human and agent participants | §4.4 kernel-mediation; no persistent inboxes |

*One litmus absorbs most of this table (three lanes derived it independently): **where does execution happen?** Anything executing off the owner's hardware, or standing between owner and kernel outside mediation, is dead on arrival.*

## D. Skips — nothing beyond what LastHarness already specs (confirmation evidence)

3 role rosters ≈ §14 tables + registry packs · 9 terse-output ≈ §7.1 output budget · 13 query-aware retrieval ≈ §6.3 bounded retrieval · 14 session persistence ≈ Session Archive + compaction · 16 Elasticsearch ≈ SQLite+FTS5 sufficient at single-owner scale · 22 editor product surface, pipeline deferred §16.4 · 34 §16.7 chose agent-browser with rationale · 42 client/server ≈ kernel/app split · 43 agent loop ≈ SDK lock §16.2 · 52 RLM/messaging previously dispositioned · 55 chat front-end ≈ Personal Agent · 56 hardware profile→pack ≈ Step 0/4 exact flow · 57 Path B endpoint named in Appendix A · 60 approval gates/parked asks already specified · 63 task-graph persistence ≈ §4.6 · 65 framework switch rejected §16.2 · 67 role-to-alias matching specified · 70 single-endpoint facade ≈ llama-swap · 76 jcode removal recorded §16 header · 79 worktree fan-out + concurrency queueing specified §16.1/§7.1 · 80 SDLC composition expressible via §14.2 roles · 84 closed beta, no documented mechanism · 85 control composition ≈ allow-lists + browser-use · 88 multiplayer boards redundant under single-owner kernel · 89 goal-state re-check adopted §14.6 · 101 surfaces exist as §4.5/§14.5/§14.6 · 102 TS mismatch; abstraction solved via aliases.

*Lane L4 adds: local-first convergence (#56/#57/#70 independently arriving at profile→download→single-endpoint) is validation evidence for Appendix A Steps 0/4/6.*

## E. Cross-lane findings (the meta-patterns)

1. **Hooks are the enforcement substrate** — ECC, TokenGoat, Context-Mode, Deepseek dsh, oh-my-openagent all implement policy as deterministic lifecycle hooks. One sentence in §16.6 prevents ad-hoc gate scattering.
2. **Decompose→gate→fan-out is one spine**, seen four ways — harvest as a family (§B above).
3. **The skills supply chain has no intake lifecycle in spec** — provenance → harness validation → versioned registration is implied by five adoptions but specified nowhere. Candidate §14.5 addendum absorbing 21/25/26/27/28/35/36/39/41/68/98/108.
4. **Verification suite is an undefined noun** — §11.3 says "run full verification suite"; checkpoints (93) and recorded eval sets (109) are the missing concreteness. Defining it once absorbs future harvests.
5. **No owner console exists in spec** — queues/logs yes, visibility surface no (77/78/15/50/71).
6. **Alias-routing is organizational, not accidental** — same indirection recurs across search, modality facades, runtimes.
7. **"Where does execution happen?" litmus** — resolves every §C conflict in one question.
8. **Local-first stack validated by convergence** — competitors keep reinventing §16.1's exact shape.

## Appendix — full coverage ledger (146 rows, entries #1–147)

| # | Tool | Disposition | One-line extract |
|---|---|---|---|
| #1 | GStack | note | Scope-forcing pre-dispatch interview (§16.9 family) |
| #2 | ECC (Everything Claude Code) | adopt | SDK lifecycle-hook gate substrate |
| #3 | Agency Agents | skip | Role rosters ≈ §14 tables + registry packs |
| #4 | TokenGoat | adopt | Tool-result injection scan at hook boundary |
| #5 | TokenGoat Mem | adopt | Staleness anchors + trust tiers for memory facts |
| #6 | Matt Pocock Skills | note | Partitioned parallel review subagents |
| #7 | Sandcastle | conflict | Sandbox abstraction over third-party runtimes/remote backends |
| #8 | Dictionary of AI Coding | note | Jargon glossary for onboarding copy consistency |
| #9 | Caveman | skip | Terse output ≈ §7.1 output-token budget |
| #10 | Ponytail | adopt | YAGNI minimal-diff ladder as dispatch instructions |
| #11 | Context-Mode | adopt | Per-call tool-output clamp (session-store half = §C conflict) |
| #12 | Headroom CLI | note | Reversible compression with restore-on-demand |
| #13 | SuperCompress | skip | Query-aware retrieval ≈ §6.3 bounded task-shaped access |
| #14 | Claude-Mem | skip | Persistence ≈ Session Archive + compaction harvests |
| #15 | Wekan | note | Read-only board render of queue/notification states |
| #16 | Elasticsearch | skip | Overkill vs SQLite FTS5 + in-process vectors at this scale |
| #17 | MoneyPrinterTurbo | note | Staged media-pipeline decomposition for video fast-follow |
| #18 | Munder-Difflin | conflict | Standing coordinated agents wrapping CLIs |
| #19 | AI Memory | conflict | Cross-agent shared memory bypassing kernel mediation |
| #20 | Open Viking | note | Unified context-namespace principle over category-keyed handles |
| #21 | Anthropic Cybersecurity Skills | adopt | MITRE/NIST-mapped security content pack |
| #22 | OpenCut | skip | Editor is product surface; pipeline deferred §16.4 |
| #23 | VoltAgent | note | Dispatch-tree observability visualization, deferred |
| #24 | TencentDB-Agent-Memory | adopt | Interaction-distillation curation job |
| #25 | Awesome Design.md | adopt | DESIGN.md brand-context injection for frontend roles |
| #26 | Official Design.md | note | Provenance-tiered skill sourcing criterion |
| #27 | Karpathy-Inspired Guidelines | adopt | Pitfall-countering instruction pack for coding roles |
| #28 | Emil Kowalski's Skills | adopt | Motion-design skill pack via skills registry |
| #29 | Deepseek Harness | adopt | Lifecycle-hook enforcement seams (consolidated with 2·44) |
| #30 | Flowise | note | Visual authoring compiling TO config, never runtime graphs |
| #31 | Remotion | note | Code-driven reproducible video rendering for fast-follow |
| #32 | Open Claude Cowork | conflict | Standing cowork shell; footprint breadth |
| #33 | Secure OpenClaw | conflict | Resident daemon on messaging channels |
| #34 | Open ChatGPT Atlas | skip | §16.7 selected agent-browser with explicit rationale |
| #35 | Awesome Claude Plugins | adopt | Vetted checksummed catalog distribution pattern |
| #36 | Awesome Codex Skills | adopt | Engine-agnostic portable skill corpus |
| #37 | Vercel Open Agents | conflict | Cloud background-agent execution model |
| #38 | Composio Open Agents | note | Credential decoupling for gateway era (hosting half = §C) |
| #39 | Browser Skills | adopt | Browser task-skill packs behind §16.7 capability |
| #40 | OpenClaw Composio Plugin | conflict | Outbound-by-default SaaS connector fabric |
| #41 | Support Skills | adopt | Domain SOP packs proving non-coding departments from packs |
| #42 | OpenCode | skip | Client/server ≈ kernel/app split; aliases cover multi-model |
| #43 | Pi | skip | Loop/unified API filled by SDK lock §16.2 |
| #44 | oh-my-openagent | adopt | Deterministic hook gates (consolidated with 2·29) |
| #46 | oh-my-pi | note | LSP diagnostics as machine-verifiable refine evidence |
| #47 | ChatGPT Codex | note | Network-off-by-default per command (delegation half = §C) |
| #48 | Claude Code | note | Plan-approval checkpoint before write phase |
| #49 | Grok Build | note | Agent-client protocol split for future owner surfaces |
| #50 | T3 Code | note | Remote owner-control surface riding §13.4 tunnels |
| #51 | MiMo Code | note | Outcome-tagged trajectory mining into proposals |
| #52 | Prime Agent | skip | RLM/messaging/session previously dispositioned in spec |
| #53 | Hermes Agent | note | Bounded learning-loop reflection through sandbox (daemon half = §C) |
| #54 | Cursor | adopt | Path-keyed scoped instruction rules (consolidated with 92) |
| #55 | BoltAI | skip | Front-end; abstraction realized by llama-swap + SDK |
| #56 | Magnitude | skip | Hardware-profile→pack flow = Appendix A Steps 0/4 exact |
| #57 | LM Studio | skip | Named Path B endpoint in Appendix A Step 1 |
| #58 | Catalyst | note | Population-based evolution search deferred on cost |
| #59 | T-search | adopt | Save-gated fresh-context retrieval rounds (+budget-phase locking note) |
| #60 | OpenWorker | skip | Gates/parked asks/deliverables already specified |
| #61 | Cloudflare Computer | conflict | Remote durable-object VFS execution |
| #62 | OpenCodex | conflict | General translation proxy + quota-evasion pools |
| #63 | Deep Agents | skip | Task-graph persistence/worktree/subagents specified |
| #64 | LangFuse | adopt | Span-per-call trace schema defined once in §16.6 Output 3 |
| #65 | LangChain | skip | Framework switch rejected §16.2 |
| #66 | LangGraph | note | Checkpoint replay/fork for recovery root-cause |
| #67 | ZCode | skip | Closed; role-to-alias matching already specified |
| #68 | Oh-my-Hermes | note | Curated interlocking bundle principle |
| #69 | local.ai | adopt | Tier-reference benchmark dataset for Step 4 tables |
| #70 | LocalStudio.ai | skip | Single-endpoint facade = llama-swap aliases |
| #71 | KittyLitter | note | Mobile owner control plane, owner-initiated only |
| #72 | Semantica | note | Provenance-edge knowledge graph enriching §12.4 archaeology |
| #73 | open-kritt | adopt | Validator-reproduced PoC pipeline for threat_detector |
| #74 | Firecrawl | note | Crawl-to-markdown ingestion if self-host ever reopens |
| #75 | HermesOffice | note | Agent-native document formats rendered losslessly |
| #76 | JCode | skip | jcode removal recorded §16 header |
| #77 | Devin AI | note | Interim progress digests during long dispatches |
| #78 | Conductor | note | Parallel-dispatch review board over live worktrees |
| #79 | Superset | skip | Worktree fan-out + bounded concurrency already spec'd |
| #80 | Vorflux | skip | Cloud SDLC execution excluded; composition via roles |
| #81 | Herdr | conflict | Always-on resident sessions vs §14.6 re-check path |
| #82 | pi-black | conflict | Credential impersonation breaks auth chain/auditability |
| #83 | OpenClaw | note | Messenger delivery-gateway adapter shape post-V1 |
| #84 | Muse Code | skip | Closed beta, no documented mechanism |
| #85 | Antigravity CLI | skip | Control composition = allow-lists + browser-use |
| #86 | Goose | note | Declarative recipe schema → job_definitions shape |
| #87 | Buzz | conflict | Peer relay channels vs kernel mediation |
| #88 | YC QM | skip | Multiplayer boards redundant under single-owner kernel |
| #89 | LongHorizon-Harness | skip | Goal-state scheduled re-check adopted §14.6 |
| #90 | SearXNG | adopt | JSON-API wired as WebSearch behind role allow-list |
| #91 | Aider | adopt | Git commit-per-changeset inside worktree runs |
| #92 | Windsurf | adopt | Repo-level rules-file injection (consolidated with 54) |
| #93 | Cline | adopt | Shadow-git step checkpoints for cheap rollback |
| #94 | Exa | note | Domain-partitioned individually allow-listable search tools |
| #95 | Tavily | note | Cleaned-extract response shape to benchmark against |
| #96 | Competitive Ads Extractor | note | Canned mission templates for recurring intelligence work |
| #97 | Deep Research Skills | adopt | Approved outline gating parallel fan-out children |
| #98 | Agent Skills | note | Dept-aligned versioned pack layout |
| #99 | ElevenLabs Skills | note | Gateway-wrapped vendor-API skill pattern for audio modality |
| #100 | Scrapling | adopt | Adaptive selector healing for scheduled scrapers |
| #101 | Odysseus AI | skip | Surfaces exist as §4.5/§14.5/§14.6 — competitive teardown instead |
| #102 | Vercel AI SDK | skip | TS mismatch; provider abstraction solved via aliases |
| #103 | Lemonade AI | note | NPU serving path extending low-tier coverage |
| #104 | AnyDoc | adopt | Canonical-markdown ingestion for Document Store |
| #105 | OpenTag | note | Chat-channel task adapter defining post-V1 messenger shape |
| #106 | LocalAI | note | Multi-backend facade if modality services multiply |
| #107 | pi-rlm | note | Chunked child delegation mapping RLM onto legal children |
| #108 | Google Skills | adopt | Harness-validated skill intake gate into registry |
| #109 | Google Agents CLI | note | Recorded eval sets concretizing §11.3 verification suite |
| #110 | ScrapeGraphAI | note | Graph-defined LLM scraping pipelines; self-hosted, complements adopted Scrapling healing (§14.3/§14.6) |
| #111 | ViMax | note | Multi-agent narrative pipeline blueprint for §16.4 video-gen fast-follow; gen APIs remote → egress-gated opt-in only |
| #112 | Galileo AI → Stitch | note | DESIGN.md/MCP export direction reinforces adopted #25 injection; closed Google Labs cloud product — reference only |
| #113 | DeerFlow | adopt | Lazy skills-as-markdown loading (load-when-needed context economy) refines §16.5 input 6; harness itself skipped per SDK lock |
| #114 | trycompai/crm | note | Evidence-ledger-over-confidence-scores pattern maps to §12 decision-log philosophy; CRM = §16.8 department-pack candidate |
| #115 | Handle | skip | Unidentified (name-only from video frame); re-verify before any extraction |
| #116 | Capptivo | skip | Demo-recorder product surface; no mechanism beyond existing §16.7 capture capability |
| #117 | Yap | adopt | OS-native speech-API reuse (delegate to OS-shipped models, zero weights loaded) informs Personal Agent voice input (§4.5) |
| #118 | Roboflow RF-DETR | note | Distilled real-time detection model — Vision-pack candidate for §17.3 Step 4 tables; Apache-2.0 local inference |
| #119 | Meetily | note | Bot-free local capture + local-LLM summarization pattern; future owner-meeting ingestion surface |
| #120 | FluidVoice | note | Multi-engine local STT zoo + Command Mode voice-control hints; enhancement runtime is proprietary — flag |
| #121 | Needle 2 | adopt | Trained-at-2bit tiny tool-caller with confidence threshold + escalate-below maps to §14.4 cost routing and Low-tier packs |
| #122 | SpiderFoot | note | YAML correlation-rules engine (declarative evidence→risk matching) extractable for §14.7 threat_detector |
| #123 | Scientific Agent Skills | adopt | Science-domain content through adopted harness-validated intake (#108 pattern); feeds Research dept roles |
| #124 | ERPNext | note | Candidate seed for future Ops department pack via §16.8; no core mechanism beyond that |
| #125 | ThreeUI | note | Components copyable-as-prompts + shipped skills reinforces prompt-artifact distribution (cf. #25 DESIGN.md) |
| #126 | Lunar | note | Pipeline-based headless commerce engine; future Commerce department-pack candidate |
| #127 | holaOS | conflict | Shared persistent cross-agent memory + messaging channels violate §6.2/§4.4/§16.5/§16.4 — competitive teardown target |
| #128 | Qiaomu-seo | note | Evidence-graded conclusions (observed/inferred/missing) + stale-source registry mirror §12/§14.7 evidence culture |
| #129 | AgentReach | skip | Identity/stats unverifiable (claimed 68k★ not found) — re-verify before any extraction; mechanism would map to §13.4 gateway |
| #130 | PocketBase | note | Single-file zero-dependency deployment shape validates §16.1 installer minimalism; SQLite-backed so no stack conflict |
| #131 | img2threejs | note | Code-only procedural 3D reconstruction with fail-closed quality gate BEFORE codegen — §11.3-adjacent evidence-gate family |
| #132 | Impeccable | adopt | Detector-rules + DESIGN.md-native design language through skills intake; strengthens adopted #25 injection |
| #133 | Hunyuan3D | note | Local-weights 3D asset/scene generation family — strengthens Media Production parked case; custom community license requires review before reuse |
| #134 | Ox Alpha | note | Stealth cloud reasoning model: Path-A evaluation option only (route-dependent retention, anonymous provider); alias-routing fits §16.1 if ever wired |
| #135 | Vaultwarden | adopt | Self-hosted credentials custody = concrete Security-Vault (§14.5) reference implementation; AGPL self-host OK |
| #136 | Agent Zero | conflict | Docker-hosted computer execution violates §16.1 no-third-party-runtimes; tool-creation + persistence patterns extractable |
| #137 | Hermes Studio | adopt | Owner-console prior art for proposal #1 — session/job/chat renders over an existing agent's state |
| #138 | LoopX | adopt | Strongest R11 handoff-packet prior art: durable goals + governed handoffs + evidence logs as first-class artifacts |
| #139 | CodeGraph | adopt | Local-first pre-indexed code KG with change-sync — upgrades §6.3 code_index/repo-map family |
| #140 | AgentMemory | note | Benchmark-topped persistent memory — compare against adopted staleness-anchor design (#5) before any swap |
| #141 | QwenPaw | skip | Odysseus/holaOS competitive cluster; surfaces exist as §4.5/§14.5/§14.6 |
| #142 | AstrBot | conflict | Multi-IM gateway = standing channel listeners (§16.4/§16.5); adapter shape already deferred post-V1 |
| #143 | Ruflo | conflict | Standing multi-player swarm coordination violates §16.5/§4.4; swarm-coordination patterns extractable as design notes |
| #144 | 9router | conflict | Quota-evasion token routing — same class as #62 ruling; Path-B singular-endpoint rule holds |
| #145 | cmux | note | License-null (flagged); terminal UX for agent sessions = owner-console render evidence (R7) |
| #146 | Qwen Code | skip | Competing terminal coding agent; SDK-lock (§16.2) makes it reference-only |
| #147 | Codebuff + Freebuff | adopt | Typed `AgentDefinition` with zod `inputSchema`/`outputSchema` + per-agent `publisher`/`spawnableAgents`/`compactContext`/`outputMode` is the canonical skill/agent artifact shape for §14.5 skills registry and §16.6 per-role schemas (Cloud product excluded by §16.1; 150-agent marketplace proves the shape scales; `99-corrections` lineage resolution logged) |

*Ledger ends — 146 rows, entries #1–147. Waves: #110–117 (h1), #118–128 (h2), #129–130 (h3), #132–136 drain-batch (2026-08-24/25), #137–146 wave-5 (2026-08-25), #147 wave-6 (2026-08-30, codebuff/freebuff lineage resolution).*

**Row-retention note (corrected 2026-08-31).** #45 (oh-my-opencode ≡ #44) is the only entry number with **no row** — it was fully merged. #38, #115, and #129 **do** still have rows despite being merged/delisted in the corrections history: they are retained deliberately as audit trail, per that ledger's own "dossier retained as audit trail" note. Read their dispositions as historical, not as live coverage claims. An earlier version of this footnote said "#45/#38 merged," which implied #38 was also absent; it is not.

---

## F. Wave-6 addendum — Codebuff + Freebuff (#147, added 2026-08-30)

Inline dossier: [`01-coding-agents.md`](archive/01-coding-agents.md) §"#147 Codebuff + Freebuff — Inline Dossier". Single-registry intake covers both products because `CodebuffAI/codebuff` is a 301 redirect to `CodebuffAI/freebuff` (one canonical repo, same HEAD, same license).

**Key adopt-landings (top 3):**

- **Per-agent `outputMode: 'structured_output'` + zod `outputSchema`** — the canonical §16.6 per-role result schema shape. Codebuff ships this in production today (see `agents/librarian/librarian.ts` returning `{ answer, relevantFiles, cloneDir }`). Pattern-donor only.
- **Per-agent `compactContext` + `windowedFileReads` flags** — moves §7.1's `context_compaction_threshold` from system-wide to per-role. Pairs with proposal #3 (Context Economy Service). Opt-in/opt-out per role without breaking the contract for others.
- **`publisher` field on every agent + `.agents/` folder convention + 150-agent marketplace** — the concrete shape for §14.5's skills category and proposal #2 (Skills Registry & Supply Chain). A skill is a *package*, not a server.

**Meta-pattern addition (extends §E):**

9. **Typed artifact beats typed protocol** — across waves 5–6, every adoptable mechanism is a *typed single-object artifact* (LoopX typed handoffs, Codebuff typed `AgentDefinition`, OpenAgent typed tool defs). Spec language should specify shapes, not protocols — the artifact *is* the contract. LastHarness's `department.config.yaml` + per-role config slice + per-skill registration object are all already in this shape.

**Conflict addition (extends §C):**

- Freebuff Cloud (`codebuff.com/store` + `freebuff.com` "Run agents on any GitHub repository") — at the **user-install surface**, this is the §16.1 cloud-worker exclusion class alongside #37 Vercel Open Agents, #38 Composio hosted half, #47 ChatGPT Codex delegation, #61 Cloudflare Computer, #77 Devin, #80 Vorflux. At the **company-operated surface** (BAROOD-AI / LASTHARNESS paid tiers and ad-supported tiers), this is *not* a conflict and is in fact the pattern-donor for company-hosted LastHarness instances. The §C table tracks the user-install class only; the company surface needs its own spec line. *(Wave-6 dossier correction logged 2026-08-30: initial draft overclaimed this as a flat conflict.)*

**User-install vs company-operated distinction (recording — spec gap flagged for §17.1):**

The spec is clear that each LastHarness *user install* is self-hosted (§2.1) and runs inside a single Linux boundary (§16.1) on the user's hardware. The spec is silent on whether BAROOD-AI / LASTHARNESS as a company may operate *hosted* LastHarness instances as a paid tier or ad-supported tier on company infrastructure. If those tiers are planned, the spec needs:

1. A separation between **user-install semantics** (self-hosted, all data local, §16.1 cloud-worker excluded) and **company-hosted semantics** (company-operated, the spec's privacy posture becomes the *company's* promise to the user, cloud execution is the default).
2. A spec line on what does and doesn't change between the two — likely *most* of §16.1/§16.6/§14.5/§12 still holds (kernel-locality, deny-by-default tool lists, decision-log immutability) but the *runtime location* changes.
3. A spec line on the **decision-log ownership** in the hosted case — the company sees logs, the user owns them, or both?

This gap is **not blocking** the user-install product but is **blocking** the company-tier revenue model. Surfacing here as the most material spec gap in the wave-6 dossier.

**Notable absence:** Codebuff has no decision log / immutable audit trail. LastHarness's §12.1 is *ahead* of the field here, not behind — the absence is a gap that LastHarness should not paper over by adopting Codebuff's pattern.
