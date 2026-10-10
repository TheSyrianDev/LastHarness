# Deep Research Skills — Feature Dossier

THIN-SOURCE
Registry: #97 | Category: 05 | Verified: MIT (README License section); star count/last-push unverified; exact pack installed locally and inspected as primary source

## Feature inventory
- Two-phase research workflow: `/research` outline generation (items + fields) then `/research-deep` parallel per-item investigation [VERIFIED-fetched:https://raw.githubusercontent.com/Weizhena/deep-research-skills/master/README.md]
- `/research` produces `outline.yaml` (items + execution config: batch_size, items_per_agent, output_dir) and `fields.yaml` (field definitions with brief/moderate/detailed levels + reserved `uncertain` list), each step confirmed by human-in-the-loop prompts [VERIFIED-fetched:~/.config/opencode/skills/research/SKILL.md]
- Extensible mid-flight: `/research-add-items` and `/research-add-fields` merge into existing outline without restarting [VERIFIED-fetched:https://raw.githubusercontent.com/Weizhena/deep-research-skills/master/README.md]
- `/research-deep` launches background web-search sub-agents in batches, one JSON output file per item, with resume support (completed item JSONs are skipped on rerun) and user approval required before each next batch [VERIFIED-fetched:~/.config/opencode/skills/research-deep/SKILL.md]
- Hard-constraint prompt templates: fixed prompt skeletons with only `{variables}` substituted, so behavior is reproducible across runs/models [VERIFIED-fetched:~/.config/opencode/skills/research-deep/SKILL.md]
- `[uncertain]` marking convention: agents must tag unverifiable field values inline and list them in an `uncertain` array instead of guessing [VERIFIED-fetched:~/.config/opencode/skills/research-deep/SKILL.md]
- Machine validation gate: post-output `validate_json.py -f fields.yaml -j result.json` must pass complete-field coverage before a task counts as done [VERIFIED-fetched:~/.config/opencode/skills/research-deep/SKILL.md]
- `/research-report` generates (then executes) a Python script that renders all JSON results into one markdown report: anchored TOC with user-selected summary fields, category multi-language mapping, complex-value formatting, and systematic skipping of uncertain values [VERIFIED-fetched:~/.config/opencode/skills/research-report/SKILL.md]
- Dedicated background web-search agent (`web-search-agent.md` + modules) separate from the main conversation loop; supports Claude Code, OpenCode (needs `OPENCODE_ENABLE_EXA=1`), and Codex, EN/ZH variants [VERIFIED-fetched:https://raw.githubusercontent.com/Weizhena/deep-research-skills/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Two-phase outline→deep workflow | 🧩 Mechanism only | §research-method | Pure orchestration pattern: plan artifacts (outline/fields YAML) separated from execution; works inside existing dept workers |
| Human-in-the-loop confirm gates between phases | ⚙ Feature adoption | §approval-workflow | Matches deny-by-default posture: batch launch requires explicit user approval |
| Resume-by-artifact (skip completed JSONs) | 🧩 Mechanism only | §worker-runtime | Idempotent ephemeral workers keyed to output files; no standing state needed |
| Hard-constraint prompt templates | 🧩 Mechanism only | §prompt-engineering | Reproducibility pattern for any dept that issues web-research work |
| `[uncertain]` + uncertainty array convention | 🧩 Mechanism only | §evidence-schema | Honest-gap marking feeds directly into decision-log evidence quality |
| Post-run validation script gate | ⚙ Feature adoption | §worker-runtime (completion checks) | Workers only report done when a machine check passes; cheap, high-value |
| Report generator script (TOC + uncertain skip) | ⚙ Feature adoption | §research/reporting | Owned by existing research function; pure local file generation |
| Background web-search agent | ⚙ Feature adoption | §egress-gateway + worker runtime | Execution happens locally; web access only through LastHarness default-deny egress allow-list |
| Multi-harness portability (Claude/OpenCode/Codex) | 🧩 Mechanism only | §skill-format | Confirms plain-Markdown skill packs are harness-portable |

## Notable absences
No sandboxing of fetched content beyond prompt discipline; web search depends on host harness capabilities (Exa flag) rather than a governed egress layer; no persistent memory or dedup across separate research topics; no citation-lineage verification of sources.
