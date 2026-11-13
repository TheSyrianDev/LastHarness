# Compatibility plan: skills and extensions from other tools (proposal)

Status: sources read in this pass (shallow clones in scratchpad only). Verified facts are marked; the rest is a proposal.

## Sources read
- agentskills.io spec: `agentskills/docs/specification.mdx`.
- Pi (`badlogic/pi-mono`): `packages/coding-agent/src/core/extensions/` (`types.ts` ExtensionAPI at about line 1717 for `registerTool`, `loader.ts` uses jiti).
- OMP (`can1357/oh-my-pi`): `src/extensions/` (mcp, codemode, tool-search as examples of `pi.registerTool`).
- DSH (`deepseek-ai/deepseek-harness`): `docs/architecture.md`, `packages/boot/plugin-manager` (README and package.json; no manifest schema read).
- Hermes (`NousResearch/hermes-agent`): skill loading not found by grep for `SKILL.md`; no Hermes skill format verified.

## Verified facts
- SKILL.md: YAML frontmatter plus Markdown body. `name` required: 1-64 chars, a-z 0-9 and hyphens, no leading, trailing or doubled hyphen, must equal the parent folder name. `description` required. Optional: `license`, `compatibility`, `metadata`, `allowed-tools`. Optional folders `scripts/`, `references/`, `assets/`; progressive loading (body first, resources on demand).
- Pi extensions: TypeScript modules loaded through jiti (`loader.ts`). API surface is `pi.on(event, handler)` for lifecycle events (session_start, session_compact, session_shutdown, and more) and `pi.registerTool(...)`. Needs a JS/TS runtime to run.
- OMP extensions: same shape (`src/extensions/*/index.ts`, `pi.registerTool`), built on Pi's runtime.
- DSH: a plugin tree of layered bundles on a Cordis framework; plugins contribute services, typed events and reversible effects. Requires the Node/Electron runtime. Plugin manifest format not confirmed in this pass.

## Ranking by cost (cheapest first)
| Format | Verdict | Why | Cost |
|---|---|---|---|
| SKILL.md (agentskills.io) | Loads directly | Plain markdown plus frontmatter; our `use_skill` already reads markdown. Validate `name` rules; map `allowed-tools` onto our tool names. `scripts/` only runs with approval. | S |
| Pi/OMP extension | Needs a JS runtime | Behaviour lives in TS code calling `pi.on` and `pi.registerTool`. Needs an embedded JS engine and an API shim. | L |
| DSH plugin | Not feasible now | Needs the Cordis runtime and Node; the plugin unit is a whole runtime, not a skill. | XL |

## Rules
- Code from another tool is off by default and asks first, as with outside CLI workers (`crates/lh-team/src/external.rs`).
- Imported skills start as proposals in Learn, not active.

## Next step
- Build a read-only SKILL.md importer that validates `name`/`description` and lists what it would load. No script runs.
