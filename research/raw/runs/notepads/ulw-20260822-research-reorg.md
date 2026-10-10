# Ultrawork Notepad — Reorganize Current Research.md into department-aligned knowledge base
Started: 2026-08-22T14:36Z

## Skill survey (bootstrap step 0)
- ultrawork — governing mode for this run (read directly from ~/.config/opencode/skills/; loader can't see new skills this session).
- ulw-research — APPLIES: classification of 109 obscure/mixed tools requires fan-out verification lanes; adapting team_create→parallel background `task` subagents (opencode surface has no team tools); journal in .omo/ulw-research/20260822-143607/.
- ulw-plan — APPLIES (CLEAR intent path): explore first, ask only surviving forks, approval gate, then ONE decision-complete plan at .omo/plans/research-reorg.md before execution.
- Skipped: none other applicable (research*/design* skills target web-research reports and UI, not registry reorganization).

## Tier triage
LIGHT — change set is documentation restructuring only; no code behavior, no auth/integration/schema surface. Known pattern, no module-boundary decisions. Consequence: 1–2 success criteria, self-review in this notepad, no reviewer loop even though a plan file will exist.

## Plan (exhaustively detailed)
1. [x] Parse Current Research.md table → canonical JSON (SESSION_DIR/tools.json). 109 rows confirmed.
2. [ ] Init draft (.omo/drafts/research-reorg.md) + plan skeleton.
3. [ ] Approval brief: forks = layout, category scheme, archive policy (+ ulw-research format gate folded in). WAIT for user.
4. [ ] After okay: write .omo/plans/research-reorg.md (scaffold headers verbatim), fill todos.
5. [ ] RED: write verify script (reconciliation old↔new + field-completeness), run → must FAIL now (no new files).
6. [ ] Classification wave: 6 parallel background general-subagent lanes (~18 tools each), fixed taxonomy, evidence basis required per row. Journal each return to expansion-log.md.
7. [ ] Skeptic pass (lead): cross-lane dupe check (#44/#45 known), conflict resolution, status flags finalized. debate-log.md row per resolution.
8. [ ] GREEN: build Research/ deliverable(s) per approved layout; rerun verify script → PASS. Evidence to SESSION_DIR/evidence/.
9. [ ] Self-review vs criteria; cleanup receipts; final summary w/ closing briefing (sources, minutes).

## Success criteria + QA scenarios
- C1 (completeness): Every original #1–#109 appears exactly once across new structure OR once in corrections/dedupe ledger with reason. VERIFY: `verify-research.ps1` exit 0 + reconciliation table printed. RED now (deliverable absent), GREEN after step 8.
- C2 (field completeness): every entry row carries Purpose ≤12 words + Category + Dept-fit + Status + Confidence; no empty cells except allowed `-` links. VERIFY: same script field check.

STOP WHEN: both criteria PASS with captured evidence + cleanup receipts recorded.

## Now
Step 3 — presenting approval brief.

## Todo
Steps 2–9 above (mirrored into todo tool).

## Findings
- Project scope: BAROOD AI = self-hosted dept-based AI OS (Docs/barood-ai.md §1–§17): kernel + 6 departments (Development/Research/Intelligence/Information/Automation/Security), Claude Agent SDK subagent workers, local llama-server/llama-swap inference, SQLite immutable decision log. Research table exists to feed dept/tooling decisions.
- Table facts: 109 rows; #44≡#45 same repo (oh-my-openagent renamed from oh-my-opencode); 9 rows have no GitHub link (closed-source or web-only); #67 ZCode official link but no repo; header claims last verified 2026-08-20; changelog section documents prior dedupe pass.
- SESSION_DIR=.omo/ulw-research/20260822-143607 (timestamp = run clock).

## Learnings
- opencode surface lacks senpi's eval/team/create_goal/todo tools → mapped: goal→# Goal block, todos→todo tool, team→background task lanes, eval-cell→batched PowerShell.
## Now
DONE — all criteria PASS.

## Todo
All steps complete (see todo tool history).

## Findings (appended)
- Deliverable: research/surveys/00-index.md + 8 category files + 99-corrections.md + archive/. Current Research.md = stub. 108 entries + 1 merge = 109 reconciled.
- Category counts: coding-agents 31, memory-context 10, search-data 7, local-inference 5, skills-plugins 24, orchestration 16, media-apps 12, infra-data 3.
- Lane output truncation happened twice (L1 lost #1-3, L2 lost #20-27) — recovered via task_id resume demands. Schema lesson: task_id resume still requires ALL required fields incl. description.

## Self-review (LIGHT tier — reviewer loop not triggered)
- Re-read deliverable diffs via verifier: C1 reconciliation exact (109 accounted, no dups), C2 field-completeness clean.
- Both criteria hold; tier held LIGHT throughout (docs-only change set). No cleanup debt.

## Cleanup receipts
- cleanup: rm -rf %TEMP%\opencode\omo (skill clone) — done earlier this session, receipt in transcript.
- cleanup: no servers/tmux/browser/temp sockets spawned by QA; SESSION_DIR artifacts (tools.json, verify script, evidence-red/green.txt, journals) retained intentionally as durable run records.

## Findings (wave 2)
- Extraction deliverable: research/extraction/09-feature-extraction.md — 29 adopt / 40 design-note / 12 conflict / 27 skip across all 108 active entries.
- Top cross-lane patterns: hooks-as-gate-substrate; decompose-gate-fanout family; skills-intake lifecycle gap in spec; verification-suite undefined noun; no owner console; alias-routing generalization; where-does-execution-happen litmus.
- Verifier extended with C3 (extraction coverage) — category-file scan excludes 09 to keep C1 counts clean.

## Self-review (wave 2, LIGHT tier)
- C1+C2+C3 all GREEN in single verifier run; consolidation decisions recorded in debate-log.md.
- Cleanup: no new runtime artifacts; SESSION_DIR journals retained deliberately.

## Findings (wave 3)
- Harvest entries verified: registry now 116 active + 1 merge; index counts corrected post-move.
- Instagram-handle-as-GitHub-org trap: you-dont-know-ai was the poster handle, not ViMax's org (HKUDS). Watch for this pattern in future social harvests.
- Two distinct products named Yap exist; caption fingerprint (Apple Speech, Mac-only) disambiguated to FrigadeHQ/yap.

## Findings (wave 4)
- Registry now 127 active + 1 merge. Wave-2 registrations #118-128: RF-DETR(118), Meetily(119), FluidVoice(120), Needle 2(121), SpiderFoot(122), K-Dense sci-skills(123), ERPNext(124), ThreeUI(125), Lunar(126), holaOS(127), Qiaomu-seo(128).
- Dispositions: adopt x2 (#121 confidence-gated tiny tool-caller -> §14.4/Low-tier; #123 science skill content), note x8, conflict x1 (#127 holaOS shared-memory).
- Shell switched PowerShell->bash mid-session; python heredoc patching used for multi-file edits.
