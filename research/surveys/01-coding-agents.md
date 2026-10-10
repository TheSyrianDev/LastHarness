# 01 · Coding Agents & Harnesses

Terminal/CLI agents, IDE agents, cloud/managed agents, and multi-agent control surfaces — the largest cluster in the registry. Context for LastHarness §16.5 (every worker is a Claude Agent SDK subagent; no wrapped external CLI).

## Terminal / CLI agents & harnesses

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #29 | Deepseek Harness | DeepSeek's open-source plugin-based agent harness (dsh) | Dev | ok | high | - | https://github.com/deepseek-ai/deepseek-harness |
| #42 | OpenCode | Open-source multi-model terminal AI coding agent (client/server) | Dev | ok | high | https://opencode.ai | https://github.com/anomalyco/opencode |
| #43 | Pi | Agent toolkit: unified LLM API, agent loop, TUI coding CLI | Dev | ok | high | https://pi.dev | https://github.com/earendil-works/pi |
| #46 | oh-my-pi | Enhanced pi terminal coding agent: hash-anchored edits, LSP, subagents | Dev | ok | high | - | https://github.com/can1357/oh-my-pi |
| #47 | ChatGPT Codex | OpenAI's open-source agentic coding CLI plus cloud agent service | Dev | ok | high | https://openai.com/codex | https://github.com/openai/codex |
| #48 | Claude Code | Anthropic's agentic coding CLI for terminals | Dev | ok | high | https://claude.com/product/claude-code | https://github.com/anthropics/claude-code |
| #49 | Grok Build | xAI's fullscreen interactive coding agent harness and TUI | Dev | ok | high | - | https://github.com/xai-org/grok-build |
| #51 | MiMo Code | Xiaomi's open agentic coding CLI; models and agents co-evolve | Dev | ok | med | - | https://github.com/XiaomiMiMo/MiMo-Code |
| #52 | Prime Agent | Self-improving RLM agent for coding workflows and autonomous tasks | Dev,Res | ok | high | https://www.primeintellect.ai | https://github.com/PrimeIntellect-ai/prime-agent |
| #56 | Magnitude | Open-source offline agent with built-in local models and hardware profiling | Dev,Res | ok | high | - | https://github.com/magnitudedev/magnitude |
| #62 | OpenCodex | Local proxy letting Codex/Claude Code run any LLM provider | Dev,Intel | ok | med | - | https://github.com/lidge-jun/opencodex |
| #67 | ZCode | Z.ai's official desktop coding-agent harness tuned for GLM-5.3 | Dev | no-repo(closed) | high | https://zcode.z.ai | - |
| #76 | JCode | RAM-efficient Rust terminal coding-agent harness; memory graph, swarm, multi-provider | Dev | ok | high | - | https://github.com/1jehuang/jcode |
| #81 | Herdr | Background terminal runtime/multiplexer keeping coding agents always-on | Dev | ok | high | https://herdr.dev | https://github.com/herdrdev/herdr |
| #82 | pi-black | Unofficial package routing Claude-subscription OAuth into Pi coding agent | Dev,Intel | ok | high | - | https://github.com/paoloanzn/pi-black |
| #84 | Muse Code | Meta's terminal coding agent powered by Muse Spark 1.2 (beta) | Dev | no-repo(closed) | high | https://musecodes.io | - |
| #85 | Antigravity CLI | Google Antigravity agentic IDE's command-line coding agent | Dev | ok | high | https://antigravity.google | https://github.com/google-antigravity/antigravity-cli |
| #86 | Goose | Open-source extensible general-purpose AI agent (desktop/CLI/API), any LLM | Dev | ok | high | https://goose-docs.ai | https://github.com/aaif-goose/goose |
| #89 | LongHorizon-Harness | Long-horizon computer-use loop harness wrapping Claude Code/Codex/OpenCode | Dev,Res | ok | high | https://lh-harness.pages.dev | https://github.com/AMAP-ML/LongHorizon-Harness |
| #91 | Aider | Terminal AI pair programmer with git-integrated edits | Dev | ok | high | https://aider.chat | https://github.com/Aider-AI/aider |

## IDE / editor agents

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #54 | Cursor | AI-first VS Code fork editor with agentic coding | Dev | ok | high | https://cursor.com | https://github.com/cursor/cursor |
| #92 | Windsurf | Agentic AI IDE (Cascade) for software development | Dev | ok | high | https://windsurf.com | - |
| #93 | Cline | Open-source autonomous coding agent extension for VS Code | Dev | ok | high | https://cline.bot | https://github.com/cline/cline |

## Cloud / managed agents

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #37 | Vercel Open Agents | Vercel template/reference app for cloud background coding agents | Dev | ok | high | - | https://github.com/vercel-labs/open-agents |
| #77 | Devin AI | Autonomous cloud software-engineer agent (Cognition Labs) | Dev | no-repo(closed) | high | https://devin.ai | - |
| #80 | Vorflux | Cloud "autopilot" coding-agent platform automating full SDLC (Rippling cofounder) | Dev | no-repo(closed) | high | https://vorflux.com | - |

## Multi-agent surfaces & companions

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #50 | T3 Code | Control surface managing local coding agents via mobile/web/desktop apps | Dev,Product | ok | high | https://t3.codes | https://github.com/pingdotgg/t3code |
| #53 | Hermes Agent | Self-improving personal agent: learning loop, memory, messaging platforms, scheduler | Dev,Auto,Info | ok | high | https://hermes-agent.nousresearch.com | https://github.com/NousResearch/hermes-agent |
| #71 | KittyLitter | Native mobile client driving Codex/Claude/OpenCode/Pi/Droid coding agents remotely | Dev | ok | high | https://kittylitter.app | https://github.com/0xSero/litter |
| #78 | Conductor | Mac app running parallel Claude Code/Codex agents in git worktrees | Dev | no-repo(closed) | high | https://conductor.build | - |
| #79 | Superset | Agentic IDE orchestrating 100+ CLI coding agents in isolated worktrees | Dev | ok | high | https://superset.sh | https://github.com/superset-sh/superset |

| #145 | cmux | Ghostty-based macOS terminal with tabs + notifications built for coding-agent sessions | Dev | license-null | high | - | https://github.com/manaflow-ai/cmux |
| #146 | Qwen Code | Alibaba's open-source terminal AI coding agent (Apache-2.0) | Dev | ok | high | https://github.com/QwenLM/qwen-code | https://github.com/QwenLM/qwen-code |

**Flags & notes**
- **#76 JCode**: this is the external CLI whose removal from LastHarness's architecture is recorded in docs/archive/lastharness-v1-spec.md §16 ("Coding worker… jcode is no longer part of the architecture"). Kept for reference.
- **#82 pi-black**: ToS-gray-area shim impersonating Claude Code wire protocol — security/compliance flag before any adoption (see 99-corrections).
- **#84 Muse Code**: musecodes.io is an unofficial fan site; product itself closed beta.
- **#79 Superset ≠ Apache Superset** (BI engine); **#86 Goose** moved orgs (block → aaif-goose, Linux Foundation/AAIF) — links current.
- Parallel-agent orchestration quartet #76/#78/#79/#81: distinct products (TUI harness / macOS app / Electron IDE / tmux-like runtime).
- **#53 Hermes Agent**: ~235k★ verified genuine via API 2026-08-24 (earlier inflation flag RESOLVED — same mega-repo class as mattpocock/skills).

---

## #147 Codebuff + Freebuff — Inline Dossier (CodebuffAI/freebuff lineage)

| Field | Value |
|---|---|
| Registry ID | #147 |
| Lineage | `CodebuffAI/codebuff` (legacy alias, redirects) → `CodebuffAI/freebuff` (canonical) — same HEAD, same branch tree, both Apache-2.0 |
| Canonical repo | https://github.com/CodebuffAI/freebuff |
| Site | https://codebuff.com (framework) · https://freebuff.com (free tier) · https://www.codebuff.com/store (agent marketplace, 150 agents) |
| Stars / Forks | ~11,232★ / 1,204 forks (GitHub API 2026-08-30, identical on both URLs — single canonical repo) |
| License | Apache-2.0 (both repos) |
| Pushed | 2026-08-30 (active, daily commits) |
| Created | 2024-07-09 |
| SDK | `@codebuff/sdk` v0.10.7 on npm, Apache-2.0, GitHub-OIDC-trusted-publisher, requires Node ≥18 |
| Dept-fit | Dev · Res · Intel · Product (multi-dept reach via typed agents) |
| Status | **ok** for the open framework + SDK; **Cloud product excluded** by §16.1 |
| Confidence | high (verified via GitHub API + raw README + npm registry + actual `agents/*.ts` source) |
| Dept-fit shape | One-line: open-source Apache-2.0 multi-agent coding framework with a typed `AgentDefinition` contract, a 150-entry published agent marketplace, and an SDK that exposes per-agent tool/model/IO/publisher control |

### Lineage resolution (correction-adjacent finding)

The original `00-index.md` registry intake would naturally have split this into two entries (Codebuff vs Freebuff) based on name. The reality is one repo: `CodebuffAI/codebuff` 301-redirects to `CodebuffAI/freebuff`, both repos return identical metadata on the GitHub API (same HEAD commit, same push timestamp, same description), and the README at `codebuff/main` is the Freebuff product README cross-referencing `codebuff.com`. **Tracked as one entry, mirrors Windsurf→Devin Desktop precedent (#92).** The two products on top of the same engine are:
- **Codebuff** (`codebuff.com`) — the open framework, SDK, marketplace (150 agents), and the paid agent API
- **Freebuff** (`freebuff.com`) — ad-supported free coding tier built on Codebuff; ships Desktop / CLI / Web / Cloud / Chat variants; subject to a "data may be used to train AI" clause on its included model tiers (per `freebuff.com/privacy-policy`).

### Feature inventory (verified from source)

| # | Mechanism | Source location | One-line shape |
|---|---|---|---|
| 1 | Typed `AgentDefinition` with zod `inputSchema` + `outputSchema` | `agents/types/agent-definition.ts`, `sdk/src/index.ts` | An agent is a single TS object: `id`, `publisher`, `model`, `providerOptions`, `displayName`, `spawnerPrompt`, `inputSchema` (zod), `outputMode` (`'last_message'`/`'structured_output'`), `outputSchema`, `toolNames`, `spawnableAgents`, `instructionsPrompt`, optional `handleSteps` (programmatic generator) |
| 2 | `publisher` field on every agent | `agents/constants.ts` (`export const publisher = 'codebuff'`) | Each agent carries a publisher identity — first-party agents share `'codebuff'`, third-party agents declare their own. Marketplace attribution + provenance at the artifact level |
| 3 | `spawnerPrompt` — semantic spawning contract | `agents/general-agent/general-agent.ts` | A parent agent's prompt description of *when* to spawn this child, not a tool name. Solves "agent composition via natural language, not regex/magic" |
| 4 | Enumerated `spawnableAgents` allow-list | same | Each agent declares the *exact set* of agents it can spawn — typed, deny-by-default, machine-checkable |
| 5 | Per-agent `outputMode: 'structured_output'` + `outputSchema` | `agents/librarian/librarian.ts` | Returns `{ answer, relevantFiles, cloneDir }` — typed return contract, parent gets structured output, not prose |
| 6 | Per-agent `compactContext: boolean` + `windowedFileReads: boolean` | `agents/base3.ts` | Per-agent compaction and retrieval-window flags — moves §7.1's compaction threshold and R3 bounded-retrieval gap from spec-level to per-role config |
| 7 | Per-agent `includeMessageHistory: boolean` | `agents/researcher/researcher-docs.ts` | Some spawned agents get parent context, others don't (fresh context). Opt-in per spawn, not global default |
| 8 | `providerOptions.data_collection: 'deny'` per agent | `agents/base3.ts` | Opt-out at the model-call level — a stronger stance than §16.1's "owner warned" privacy posture |
| 9 | `spawn_agent_inline` with `includeToolCall: false` | `agents/general-agent/general-agent.ts` | Subagents that run transparently inside the parent — no context noise in the parent's transcript |
| 10 | `context-pruner` as a *system-internal* tool | `agents/context-pruner.ts` (36k lines!) | A dedicated pruning agent invoked inline before each `STEP`. Same mechanism as Caveman #9 / Headroom #12, but as system internals — no proxy, no standing process |
| 11 | `getCustomToolDefinition` SDK API with zod input | `sdk/README.md` | Custom tool definitions are first-class typed objects with `toolName`, `description`, `inputSchema`, `exampleInputs`, `execute`. Tools and agents share the same artifact shape |
| 12 | `.agents/` folder convention + `loadLocalAgents()` SDK API | `sdk/README.md` | Project-local agent packs live in `.agents/`, auto-loaded at SDK init. The marketplace shape ports cleanly to a skill registry |
| 13 | Agent Store with versioned agent refs (`codebuff/base@0.0.16`) | `https://www.codebuff.com/store`, 150 agents | A registry of published agents, each versioned, each consumable by ID. Closer to npm than to MCP — agents are *packages*, not servers |
| 14 | `npx skills find` / `npx skills add` community skill distribution | `agents/constants.ts` `SKILL_DISCOVERY_GUIDANCE` | Codebuff uses the same `npx skills` distribution shape as #9 Caveman / #132 Impeccable — confirms cross-tool skill-pack convention |
| 15 | Default `.env` blocking at SDK layer | `sdk/README.md` (fileFilter section) | `.env`/`.env.*` blocked except `.env.example`/`.sample`/`.template`. `fileFilter` is an extension hook for project-specific rules. **No** default-instilled gitignore check (opt-in via no fileFilter) |
| 16 | `CodebuffClient.handleEvent(event)` event stream | `sdk/README.md` | Every tool call, every agent start/finish, every error emits a typed event. A precedent for §16.6's worker-event-stream substrate (R10) |
| 17 | `runTerminalCommand` console-free broker pattern | `docs/agents-and-tools.md` | Detached, hidden helper spawns shells with no console/interactive stdin; uses three stdio channels; self-reaps tree if parent disappears. Solves Windows PTY handshake failures |
| 18 | Knowledge files at multiple scopes | `sdk/README.md` (Knowledge Files section) | Auto-discovers `knowledge.md` / `AGENTS.md` / `CLAUDE.md` per directory + home. Rides the same convention as LastHarness's repo-map pre-step (§14.2) |
| 19 | 150 published agents in the marketplace | `https://www.codebuff.com/store` (page title) | Concrete scale of the agent-as-package model — proof that marketplace-shaped agent distribution works |
| 20 | Freebuff Cloud product | `freebuff.com` (README + product page) | "Run agents on any GitHub repository" — sandboxed execution on Freebuff infrastructure. **Direct §16.1 cloud-worker exclusion candidate** |
| 21 | `default-on` data training clause on free tiers | `freebuff.com/privacy-policy` (excerpt in README) | "Only when a model or feature says data may be used for AI training" + "data may be analyzed for ads" — opt-out via recognized signals or per-jurisdiction. Default-deny-relevant for any private deployment |
| 22 | Programmatic `handleSteps` generator function | `agents/general-agent/general-agent.ts` | Agents can be fully programmatic (JS generator that yields `{ toolName, input }` per step), not just LLM-prompted. Captures the same "context-as-managed-slots" idea as Prime Agent RLM but per-agent |
| 23 | Per-agent model picker + provider shim | `packages/llm-providers/` | Multi-provider abstraction at the framework level — equivalent to the alias-router role in LastHarness §14.4 |
| 24 | `spawn_agents` parallel-call affordance | `agents/general-agent/general-agent.ts` (`spawn multiple agents in parallel when possible`) | Parents can fire multiple sub-agents in one tool call — direct precedent for §4.6's aggregate-limit rule and R8 delegation-tree budgets |

### Spec-fit analysis (which mechanisms land, which don't)

#### Direct Adopt candidates (§16.5 / §16.6 / §14.5 / §7.1)

- **#1 typed AgentDefinition with zod input/output schemas** → reinforces §16.6's per-role result schemas addendum. LastHarness already specifies per-role schemas; Codebuff's `outputSchema` field is the canonical typed-contract shape (zod, not ad-hoc). Pattern-donor, no code reuse.
- **#2 `publisher` field on every agent** → maps onto §14.5's skills category. A skill/pack with `publisher: 'some-author'` is provenance at the artifact level — strengthens the R5 skill-pack supply chain (the single most-recurred cluster). Direct fit with proposal #2 in `11-department-blueprints.md`.
- **#4 enumerated `spawnableAgents` allow-list** → exactly R9 ("scoped tool-profiles shrinking down the delegation chain") and §16.6's deny-by-default. Type-level enumeration closes the RLM-style "wildcard spawn" loophole that was named as a §16.2 conflict.
- **#5 `outputMode: 'structured_output'` + zod `outputSchema`** → the literal §16.6 addendum mechanism, named in the spec as "per-role result schemas". Codebuff has this in production today; LastHarness should treat it as the reference implementation shape.
- **#6 `compactContext` + `windowedFileReads` per-agent flags** → moves §7.1's `context_compaction_threshold` from system-wide to per-role. Strong adoption: each role can opt into (or out of) compaction without breaking the contract for others. Pairs with proposal #3 (Context Economy Service).
- **#7 `includeMessageHistory: boolean` per-agent** → a concrete R11 (kernel-composed handoff packet) mechanism. Some spawned agents see parent context, some start fresh — typed per-spawn, not per-system.
- **#8 `providerOptions.data_collection: 'deny'` per agent** → a §16.1 privacy posture that's stronger than LastHarness's current "owner warned" stance. Mechanism-only adoption: a per-agent data-collection opt-out could harden §14.4's Intelligence recommendation discipline.
- **#11 `getCustomToolDefinition` typed tool API** → mirrors #1's agent shape. Tools and agents as the same first-class artifact — direct fit for §16.5 dispatch and §14.5 skills.
- **#12 `.agents/` folder convention + `loadLocalAgents()`** → maps onto §14.5's `skills` category and the proposed Skills Registry (#2 in `11-department-blueprints.md`). Mechanism-only — LastHarness would own the folder layout and the loader.
- **#16 `handleEvent(event)` event stream** → the R10 (append-only event streams as audit substrate) concrete shape. Already partially named in §16.6 Output 3 (transcript/trace); Codebuff's typed event schema is the prior art.
- **#18 Knowledge files at multiple scopes** → reinforces §14.2's repo-map pre-step and §6.3's department-scoped memory.
- **#22 Programmatic `handleSteps` generator** → confirms the "context-as-managed-slots" principle from Prime Agent RLM (already recorded as a §16.6 design-note) but per-agent rather than per-system. Pattern-donor only — does not conflict with §16.5's ephemeral-dispatch rule because the generator runs inside a single dispatch's lifetime.

#### Conflict / exclude

- **#20 Freebuff Cloud product** — "Run agents on any GitHub repository" on Freebuff-hosted infrastructure is a **user-install §16.1 cloud-worker exclusion**: dispatched-tool execution on remote hosts controlled by a third-party provider is not permitted in a LastHarness user install. **However**: this is only a conflict at the *user-install* surface. The spec says nothing about company-operated revenue tiers (§17.1 names "non-technical end user" but does not specify whether BAROOD-AI / LASTHARNESS as a company may operate hosted LastHarness instances on its own infrastructure as a paid or ad-supported tier). If the company plans such tiers (paid tiers like OpenCode/ChatGPT/Claude, ad-supported tiers like Freebuff), Freebuff Cloud is a pattern-donor for the *company-operated* surface, not a feature in the user-install. Cite separately; do not lump it in the §C conflict table as if the spec forbids it for the company. *(Correction noted in `99-corrections.md` 2026-08-30 — the wave-6 dossier initially overstated this.)*
- **#21 Default-on data training clause on free tiers** — the free tier's `data_collection: 'allow'` default for included models violates §2.1 ("Self-hosted and private") *at the user-install surface*. At the company-operated surface it would need its own privacy posture, separately specified. Framework itself is Apache-2.0 and self-hostable; the free-tier privacy posture is the conflict surface, not the framework.
- **Default-on anonymous telemetry in some Codebuff products** — at the user-install surface, same posture as Caveman #9 / Hermes Agent #53 — would require hard-disabling in any packaged form per §13.4.2 default-deny egress. At the company-operated surface, telemetry is expected and needs its own spec line.

#### Notable absences

- No standing process between sessions — every agent is a per-dispatch instance. **This is itself evidence for §16.5's ephemeral-dispatch rule**: Codebuff is one of the largest multi-agent products shipping today, and it chose ephemeral-per-dispatch over persistent workers. The market has voted on this design choice.
- No MCP servers in the agent definitions (only built-in `toolNames` and custom tools). Codebuff's "agents are the new MCP" claim is a *different* distribution primitive, not a replacement.
- No memory writes during dispatch. Agents return their output; the caller (SDK / parent agent) decides whether to persist. Maps onto §16.6's "Memory writes are not part of the return value" line.
- No decision log. No immutable audit trail of dispatched tasks. LastHarness's §12.1 is *ahead* of Codebuff here, not behind — the absence is a gap that LastHarness should not paper over by adopting Codebuff's pattern.

### Bottom line

**Pattern-donor only, no code reuse.** Every adoptable mechanism above is a typed-contract / per-agent flag / event-stream shape that LastHarness should reference when finalizing §16.6, §14.5, and §7.1 — not a feature to import. Codebuff's *runtime* (TypeScript + Bun, OAuth-required SDK, hosted marketplace) is wrong shape for LastHarness; Codebuff's *agent-as-artifact* model is right shape for §14.5's skills registry and §16.6's per-role schemas.

The 150-agent marketplace is the most consequential signal in this dossier: it proves that **typed, versioned, publisher-attributed agent packages can scale to 150+ entries with this artifact shape**. That's the precedent LastHarness's Skills Registry needs.
