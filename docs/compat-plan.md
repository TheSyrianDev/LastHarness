# Compatibility plan: skills and extensions from other tools (proposal)

Status: sources read 2026-11-13 (shallow clones in scratchpad only, not run). Re-checked the same day against Hermes 0e72e8a2, Pi 35019ae, OMP 5de4b03, DSH 39163dd3. Each claim is marked **[verified file:line]** or **[unverified]**. The rest is a proposal.

## Sources read
- agentskills spec: `agentskills/docs/specification.mdx` (repo `agentskills/agentskills` at 83021e2). **Not re-checked in this pass** (not in the clone set).
- Pi (`badlogic/pi-mono` at 35019ae): `packages/coding-agent/src/core/extensions/` (`types.ts`, `loader.ts`).
- OMP (`can1357/oh-my-pi` at 5de4b03): `packages/coding-agent/src/extensibility/extensions/` (`types.ts`, `loader.ts`), `packages/coding-agent/examples/extensions/`. OMP is a fork of Pi **[verified omp README.md:26]**, not a plugin on top of Pi.
- DSH (`deepseek-ai/deepseek-harness` at 39163dd3): `docs/architecture.md`. `packages/boot/plugin-manager` is present but its manifest schema is **[unverified]**.
- Hermes (`NousResearch/hermes-agent`): skill format **[verified]** (see below). The earlier note said no loader was found; that was wrong.

## Verified facts
- SKILL.md (agentskills spec): YAML frontmatter plus Markdown body. **Spec limits not re-checked this pass** (spec repo not cloned). Hermes enforces the same limits in code: `MAX_NAME_LENGTH = 64` and `MAX_DESCRIPTION_LENGTH = 1024` **[verified hermes tools/skills_tool_plugin.py:15-16]**. Hermes reads `SKILL.md` through `iter_skill_index_files(skills_root, "SKILL.md")` **[verified hermes tui_gateway/methods_profiles.py:364]**, and its skill helpers live in `agent/skill_utils.py` **[verified by file presence]**.
  - `name` required: 1-64 chars, lowercase letters, digits and hyphens, no leading or trailing hyphen, no `--`, must equal the parent folder name. **[unverified this pass]**
  - `description` required: max 1024 chars, non-empty. Length limit verified in Hermes (above).
  - Optional: `license`, `compatibility` (max 500 chars), `metadata`, `allowed-tools` (space-separated, experimental). **[unverified this pass]**
  - Optional folders `scripts/`, `references/`, `assets/`; progressive loading. **[unverified this pass]**
- Pi extensions: TypeScript modules loaded with jiti. Verified: `loader.ts:2` (header comment "using jiti"), `loader.ts:13` (type import of `createJiti`), `loader.ts:53` (`createJitiPromise` load path). The earlier `loader.ts:588` cite is **[verified: `createJiti(import.meta.url, {...})` at loader.ts:588]**.
  - API is `ExtensionAPI` at `types.ts:1638` **[verified]**. `pi.on` overloads at `types.ts:1648` (`session_start`) and `1662` (`session_compact`) **[verified]**.
  - Event types: `session_start` `types.ts:777`, `session_compact` `types.ts:820`, `session_shutdown` `types.ts:846` **[verified]**.
  - `registerTool<TParams, TDetails, TState>(tool: ToolDefinition<TParams, TDetails, TState>)` at `types.ts:1717` **[verified]**. `TState` is a generic on `ToolDefinition` at `types.ts:600` **[verified]**.
  - Needs a Node-style runtime with jiti.
- OMP extensions: TypeScript loaded with native Bun import **[verified omp loader.ts:2]**, not jiti. `registerTool<TParams, TDetails>(tool: ToolDefinition<TParams, TDetails>): void` at `types.ts:1963` **[verified]**, no `TState` **[verified types.ts:1016 and 1963]**. `ExtensionAPI` at `types.ts:1856` **[verified]**.
  - `pi.on` lifecycle hooks exist. The earlier cite `types.ts:1881` is a `resources_discover`/`session_start` overload region **[verified at types.ts:1879-1882]**.
  - Examples present: `examples/extensions/hello.ts`, `tools.ts`, `api-demo.ts`, and `with-deps/index.ts` (with its own `package.json` and lockfile) **[verified by listing]**.
  - Not found in this clone: `src/extensions/*/index.ts` and examples `mcp`, `codemode`, `tool-search` **[verified absent]**.
- DSH: Cordis-based harness **[verified docs/architecture.md:5, 9]**. Node apps launch through `dsh --profile <name>` **[verified architecture.md:43]**. Units are bundles and profiles declared in each package's `package.json` under a `dsh` field (`dsh.profile`, `dsh.bundle`) **[verified architecture.md:21]**.
  - Electron **is** in the doc: the Electron desktop app ships the dsh runtime and runs the Host in Electron Node mode **[verified architecture.md:51-53]**. The earlier "Node only, no Electron" note was wrong.
  - Patches are YAML (`cordis.patch.yml`) **[verified architecture.md:13-17, 17]**.
  - Plugin manifest schema: **[unverified]**.

## Could not read or verify
- DSH plugin manifest schema (`packages/boot/plugin-manager` present; schema not read).
- agentskills spec limits and optional fields (spec repo not in the clone set this pass).
- OMP `mcp`, `codemode`, `tool-search` examples (absent from this clone, so the claim is only "not found").

## Ranking by cost (cheapest first)
| Format | Verdict | Why | Cost |
|---|---|---|---|
| SKILL.md (agentskills spec) | Loads directly | Plain markdown plus frontmatter; Hermes already loads it with the same name and description limits. Our `use_skill` already reads markdown. Validate `name` and `description` against the limits above; map `allowed-tools` onto our tool names. `scripts/` only runs with approval. | S |
| Pi or OMP extension | Needs a JS runtime | Behaviour lives in TS code calling `pi.on` and `pi.registerTool`. Pi needs jiti; OMP needs Bun. Needs an embedded JS engine and an API shim for either. | L |
| DSH plugin or bundle | Not feasible now | Needs the Cordis runtime and Node (Electron desktop uses the same runtime); the unit is a whole profile or bundle, not a skill. | XL |

## Rules
- Code from another tool is off by default and asks first, as with outside CLI workers (`crates/lh-team/src/external.rs`). **[unverified: file not checked this pass]**
- Imported skills start as proposals in Learn, not active.

## Next step
- Build a read-only SKILL.md importer that validates `name` and `description` against the spec limits and lists what it would load. No script runs.
