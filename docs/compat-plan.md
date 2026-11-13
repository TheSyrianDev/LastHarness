# Compatibility plan: skills and extensions from other tools (proposal, not decided)

Status: planning only. Built from the local surveys (`research/surveys/05-skills-plugins.md`, `research/surveys/10-feature-dossiers/`). The agentskills.io spec, the Pi/OMP TypeScript extension API and the DSH plugin format were NOT read in this pass. Each row needs a source read before a decision.

## Ranking by cost (cheapest first)

| Format | Verdict | Why | Cost |
|---|---|---|---|
| SKILL.md (markdown with frontmatter, agentskills.io style) | Loads directly | Our skills are markdown already (`lh-learn`, `use_skill`). Needs a frontmatter reader and a mapping of tool names. | S |
| Claude-style plugin folders (commands, skills, rules) | Mostly direct | Same markdown parts; commands need a slash-command mapping. Hooks need a per-event adapter. | M |
| Pi/OMP TypeScript extensions | Needs a JS runtime | Extensions call a TS API. Running them means embedding a JS engine, a sandbox and an API shim. | L |
| DSH plugins | Not feasible yet | Format and runtime not read; treat as unknown until the spec is read. | ? |

## Rules
- Anything that runs code from another tool is off by default and asks first, same as outside CLI workers (`crates/lh-team/src/external.rs`).
- Imported skills start as Learn proposals, not active skills, so the user approves them first.

## Next step
- Read the SKILL.md spec and one real example; then build a read-only importer that lists what it would load (no run).
