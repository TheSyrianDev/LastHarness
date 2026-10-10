# TICKET: Wave-5 dossier lane (10 files)

TASK: Write 10 wave-4 feature-dossier files for newly-registered registry entries. You have file-write access. README/docs-driven ONLY — never invent a feature.

EXPECTED OUTCOME: 10 template-complete files; report status DONE / DONE_WITH_CONCERNS / BLOCKED.

CONTEXT — BAROOD locked invariants: ephemeral Claude Agent SDK workers only (no standing processes/inboxes/daemons) · ALL cross-agent routing kernel-mediated (no peer-to-peer) · deny-by-default tool allow-lists · department-scoped gated memory · immutable SQLite decision log · ONE self-hosted Linux-env boundary (NO third-party runtimes, NO cloud execution of agent workloads) · local-first inference · default-deny egress gateway · non-technical onboarding · hardware-tiered model packs.

Verdict types: 🏛 Department candidate | ⚙ Feature adoption | 🧩 Mechanism only | ❌ Conflict-skip. Litmus per feature: WHERE DOES EXECUTION HAPPEN?

Ledger dispositions already assigned — keep verdict tables consistent with them while adding feature-level detail.

## FILES + TOOLS
(Verified stats: read `.omo/ulw-research/20260824-100508/repo-verify-w5.txt` first.)

All paths relative to `research/surveys/10-feature-dossiers/wave4/`:

1. `hermes-studio.md` ← #137 EKKOLearnAI/hermes-studio | NOASSERTION license FLAG | Web dashboard for Hermes Agent (#53): multi-platform chat, session management, scheduled jobs. LEDGER: **adopt** — owner-console prior art for proposal #1.
2. `loopx.md` ← #138 huangruiteng/loopx | Apache-2.0 | Long-horizon agent control plane: durable goals, evidence logs, governed handoffs across Codex/Claude Code. LEDGER: **adopt** — strongest R11 handoff-packet prior art; DETAIL the handoff/governance mechanics thoroughly.
3. `codegraph.md` ← #139 colbymchenry/codegraph | MIT, 68k★ | Pre-indexed code knowledge graph, auto-syncs, local-first, supports Hermes Agent + Claude Code + Codex. LEDGER: **adopt** — upgrades §6.3 code_index/repo-map family; detail indexing/sync/retrieval mechanics.
4. `agentmemory.md` ← #140 rohitg00/agentmemory | Apache-2.0, 27.4k★ | "#1 persistent memory by benchmarks". LEDGER: **note** — compare architecture vs adopted staleness-anchor design (#5); what do its benchmarks actually measure?
5. `qwenpaw.md` ← #141 agentscope-ai/QwenPaw | Apache-2.0, 34.4k★ | Self-hosted personal AI assistant. LEDGER: **skip** — Odysseus/holaOS cluster; surfaces exist §4.5/§14.5/§14.6.
6. `astrbot.md` ← #142 AstrBotDevs/AstrBot | AGPL-3.0 FLAG, 39.6k★ | Multi-IM agent gateway + plugin framework; desktop sibling app exists (note within same entry). LEDGER: **conflict** — standing channel listeners violate §16.4/§16.5.
7. `ruflo.md` ← #143 ruvnet/ruflo | MIT, 69.4k★ | Agent meta-harness: multi-player swarms, autonomous coordination. LEDGER: **conflict** — standing swarm processes violate §16.5/§4.4; EXTRACT swarm-coordination patterns as design notes.
8. `9router.md` ← #144 decolua/9router | MIT, 26.3k★ | Free-token router w/ auto-fallback across providers for coding-agent CLIs. LEDGER: **conflict** — quota-evasion class (same ruling as #62); extract anything legitimate (failover ordering, endpoint health checks).
9. `cmux.md` ← #145 manaflow-ai/cmux | LICENSE-NULL FLAG, 26.5k★ | Ghostty-based macOS terminal with vertical tabs + notifications for coding agents. LEDGER: **note** — session UX = R7 owner-console render evidence.
10. `qwen-code.md` ← #146 QwenLM/qwen-code | Apache-2.0, 27.4k★ | Alibaba's terminal AI coding agent. LEDGER: **skip** — SDK-lock reference-only; inventory how it differs from Gemini CLI lineage.

## TEMPLATE (every file, exactly)

```
# <Tool> — Feature Dossier
Registry: #N | Category: 0X | Verified: <stars, license, pushed>
## Feature inventory
- <feature> [VERIFIED-fetched:<url> | REPORTED-unverified]
## Per-feature verdict
| Feature | Type | BAROOD landing | Rationale |
|---|---|---|---|
| <feature> | 🏛/⚙/🧩/❌ | §-slot or dept or — | why |
## Notable absences
```
(Add `THIN-SOURCE` after the title if docs are weak; extract less, never pad.)

## MUST DO
- Read repo-verify-w5.txt first for Verified lines.
- Fetch raw README per repo: `https://raw.githubusercontent.com/<owner>/<repo>/main/README.md` (master fallback); cite URL on every VERIFIED bullet.
- loopx and ruflo are prior-art goldmines: mechanism detail matters most there.

## MUST NOT
- Touch ANY file outside your 10 listed paths. No subagents. No edits to Docs/, top-level Research/*.md, .omo/.

## OUTPUT FORMAT
First line `DONE` / `DONE_WITH_CONCERNS` / `BLOCKED`; then one line per tool:
`#N slug | feats=<k> verified=<k> thin=YES/NO | top: <mech>; <mech>; <mech>`
then `PATTERNS:` cross-tool recurring mechanisms + anything surprising.

VERIFY before reporting: all 10 files contain "Feature Dossier" + "Per-feature verdict".
