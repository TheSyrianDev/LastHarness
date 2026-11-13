# Compatibility plan: skills and extensions from other tools (proposal)

Status: sources read 2026-11-13 (shallow clones in scratchpad only, not run). Verified facts are marked with the file and line read. The rest is a proposal.

## Sources read
- agentskills spec: `agentskills/docs/specification.mdx` (repo `agentskills/agentskills` at 83021e2).
- Pi (`badlogic/pi-mono` at 071c1e0): `packages/coding-agent/src/core/extensions/` (`types.ts`, `loader.ts`).
- OMP (`can1357/oh-my-pi` at 5de4b03): `packages/coding-agent/src/extensibility/extensions/` (`types.ts`, `loader.ts`), `packages/coding-agent/examples/extensions/`. OMP is a fork of Pi (`README.md:26`), not a plugin on top of Pi.
- DSH (`deepseek-ai/deepseek-harness` at 39163dd3): `docs/architecture.md`. Did not read `packages/boot/plugin-manager` (README or code).
- Hermes (`NousResearch/hermes-agent`): skill loading not found by grep for `SKILL.md`; no Hermes skill format verified.

## Verified facts
- SKILL.md (agentskills spec, `specification.mdx`): YAML frontmatter plus Markdown body.
  - `name` required: 1-64 chars, lowercase letters, digits and hyphens (spec says Unicode lowercase alphanumeric), no leading or trailing hyphen, no `--`, must equal the parent folder name.
  - `description` required: max 1024 chars, non-empty.
  - Optional: `license`, `compatibility` (max 500 chars), `metadata` (string-to-string map), `allowed-tools` (a space-separated string, e.g. `Bash(git:*) Bash(jq:*) Read`; experimental).
  - Optional folders `scripts/`, `references/`, `assets/`; progressive loading (name and description at startup, body on activation, resources on demand).
- Pi extensions: TypeScript modules loaded with jiti (`loader.ts:13`, `loader.ts:53`, `loader.ts:588`). API is `ExtensionAPI` (`types.ts:1638`): `pi.on(event, handler)` (`types.ts:1648`, `1662`; events include `session_start` `types.ts:777`, `session_compact` `types.ts:820`, `session_shutdown` `types.ts:846`) and `pi.registerTool` (`types.ts:1717`, generic over `TParams`, `TDetails`, `TState`). Needs a Node-style runtime with jiti.
- OMP extensions: TypeScript loaded with native Bun import (`loader.ts:2`). Not jiti. `registerTool<TParams, TDetails>(tool: ToolDefinition<...>): void` (`types.ts:1963`), no `TState`. `pi.on` lifecycle hooks exist (`types.ts:1881`). Examples are flat files (`examples/extensions/hello.ts`, `tools.ts`, `api-demo.ts`) and `examples/extensions/with-deps/index.ts`.
  - Not found: the earlier claim of `src/extensions/*/index.ts` and examples `mcp`, `codemode`, `tool-search`. Those paths do not exist in this clone.
- DSH: Cordis-based harness (`docs/architecture.md:7-9`). Node apps launch through `dsh --profile <name>` (`architecture.md:43`). Units are bundles and profiles declared in each package's `package.json` under a `dsh` field (`dsh.profile`, `dsh.bundle`, `architecture.md:21`). Patches are YAML (`cordis.patch.yml`, `architecture.md:13-17`). Not confirmed: the plugin manifest schema, and any Electron runtime (earlier draft said Node/Electron; only Node is in the doc).

## Could not read or verify
- DSH plugin manifest schema (`packages/boot/plugin-manager` not read).
- Hermes skill format (no `SKILL.md` loader found by grep).
- OMP `mcp`, `codemode`, `tool-search` example extensions (not in this clone).

## Ranking by cost (cheapest first)
| Format | Verdict | Why | Cost |
|---|---|---|---|
| SKILL.md (agentskills spec) | Loads directly | Plain markdown plus frontmatter; our `use_skill` already reads markdown. Validate `name` and `description` against the limits above; map `allowed-tools` onto our tool names. `scripts/` only runs with approval. | S |
| Pi or OMP extension | Needs a JS runtime | Behaviour lives in TS code calling `pi.on` and `pi.registerTool`. Pi needs jiti; OMP needs Bun. Needs an embedded JS engine and an API shim for either. | L |
| DSH plugin or bundle | Not feasible now | Needs the Cordis runtime and Node; the unit is a whole profile or bundle, not a skill. | XL |

## Rules
- Code from another tool is off by default and asks first, as with outside CLI workers (`crates/lh-team/src/external.rs`).
- Imported skills start as proposals in Learn, not active.

## Next step
- Build a read-only SKILL.md importer that validates `name` and `description` against the spec limits and lists what it would load. No script runs.
