# research-reorg - Work Plan

## TL;DR (For humans)
Reorganize the flat 109-tool dump in `Current Research.md` into a categorized `Research/` knowledge base aligned with BAROOD AI's six departments: `research/surveys/00-index.md` master index + one Markdown file per functional category + `research/raw/freebuff-dump-2026-08-20.md` verbatim original + `research/extraction/99-corrections.md` dedupe/corrections ledger. Every original entry #1–#109 accounted for exactly once (deliverable or ledger). Classification sourced from 6 parallel verification lanes with evidence basis per entry.

## Scope
IN: parse/verify/classify/re-render the existing 109 entries; dedupe (#44≡#45 known); flag closed-source/no-repo/unverified links; archive original verbatim; replace `Current Research.md` body with pointer stub to Research/.
OUT (Must NOT have): new tool suggestions beyond the 109; link rot re-verification beyond lane-level confidence flags; any change to `Docs/barood-ai.md`; PDF/DOCX rendering (declined at format gate); git operations (folder is not a repo).

## Verification strategy
Scripted reconciliation (`verify-research.ps1`, SESSION_DIR): (a) old-file parse yields exactly #1–#109; (b) deliverable scan finds each number exactly once across category files + corrections ledger (archive/ excluded); (c) every deliverable row carries Purpose/Category/Dept-fit/Status/Confidence fields, empty allowed only for `-` links. RED before build (files absent), GREEN after.

## Execution strategy
1. Archive original verbatim → `research/raw/`.
2. RED run of verifier against pre-build state; capture output to SESSION_DIR/evidence/.
3. One wave, 6 parallel `general` subagent lanes (#1–19, 20–37, 38–55, 56–73, 74–91, 92–109), fixed taxonomy, structured returns, evidence basis required; external fetch/search only when not confidently known.
4. Lead skeptic pass: cross-lane dupe check, conflict resolution, status finalization; debate-log rows.
5. Build `research/surveys/00-index.md` + category files + `99-corrections.md`; stub `Current Research.md`.
6. GREEN verifier run; capture PASS evidence.
7. Self-review vs success criteria (LIGHT tier); cleanup receipts; closing briefing.

## Todos
- [ ] Archive original → research/raw/freebuff-dump-2026-08-20.md
- [ ] RED verifier run captured (fails: deliverable absent)
- [ ] Wave spawn: lanes 1–6 background-classified, journal expansion-log.md per return
- [ ] Skeptic pass: dupes/conflicts finalized, debate-log.md written
- [ ] Build research/surveys/00-index.md + 8 category files + 99-corrections.md
- [ ] Stub Current Research.md → points to Research/
- [ ] GREEN verifier run captured (exit 0)
- [ ] Cleanup receipts + closing briefing

## Final verification wave
After ALL todos: rerun `verify-research.ps1` (inputs changed since last green run → rerun required), confirm exit 0, confirm no temp artifacts outside SESSION_DIR/.omo remain.

## Commit strategy
Folder is not a git repository — no commits possible. Durable artifacts live under `.omo/plans/`, `.omo/notepads/`, `.omo/ulw-research/<ts>/`. If user later inits git, this plan + SESSION_DIR journal provide the history.

## Success criteria
- C1 COMPLETENESS: every #1–#109 appears exactly once across Research/ category+corrections files (archive excluded); verifier exit 0 prints reconciliation table.
- C2 FIELD COMPLETENESS: every deliverable entry has Purpose (≤12 words), Category, Dept-fit, Status, Confidence; only `-` cells may be empty; verifier field check passes.
STOP WHEN both PASS with captured evidence + cleanup receipts recorded.
