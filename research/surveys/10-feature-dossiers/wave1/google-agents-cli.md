# Google agents-cli — Feature Dossier
Registry: #109 | Category: 06 | Verified: 5,706 stars, Apache-2.0, last push 2026-08-18

SLOT FOCUS: recorded eval sets concretizing §11.3 verification suite — eval-set format and replay commands.

## Feature inventory
- Recorded eval dataset as a committed file (`tests/eval/datasets/basic-dataset.json`) plus metrics config in `eval_config.yaml`; `agents-cli eval run` executes the agent over the dataset and grades the traces in one step [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- Generate/grade split for replay without re-inference: `eval generate --dataset <file> --output custom_traces/` records agent traces; `eval grade --metrics ... --traces custom_traces/` re-grades existing traces against any metric set without touching the agent [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- Grade a *deployed* agent by URL instead of local run: `eval run --url https://my-agent.run.app --app-name app` [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- Built-in metric catalog (`agents-cli eval metric list`) incl. `general_quality`, `instruction_following`, `tool_use_quality`, `multi_turn_tool_use_quality`, `multi_turn_trajectory_quality`, `multi_turn_task_success`, `final_response_match` (golden-reference comparison), `hallucination` (atomic-claim verification against tool-returned context), `grounding`, `safety`; only designated metrics accept multi-turn traces [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- Custom metric schemas: Code Execution Metric (local Python `def evaluate(instance) -> {score, explanation}` executed in the CLI process, no GCP required; optional `"execution": "remote"` opts into Vertex sandbox) and LLM-as-a-Judge Metric (`prompt_template` + `judge_model` + `judge_model_sampling_count` 1–32) [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- `eval dataset synthesize`: cold-start dataset generation by inspecting the local ADK agent, steered with `--instruction` / `--environment-context`, played out against an LLM-backed user simulator; output is an editable, committable `*-dataset.json` with trace pre-populated [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- `eval compare baseline_results.json candidate_results.json` side-by-side regression view; `eval analyze --eval-result grade_results.json` clusters failure modes into themes; `eval optimize` auto-tunes agent prompts from eval data (minutes-to-hours) [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- Prescribed eval-fix loop: write 1–2 core cases → run → read failures → fix → re-run → expand to edge cases; expected 5–10+ cycles before consistent pass [VERIFIED-fetched:https://google.github.io/agents-cli/guide/evaluation/]
- Skills installable into third-party coding agents (`npx skills add google/agents-cli`) covering lifecycle, ADK code, scaffold, eval methodology, deploy, publish, observability; CLI usable standalone without a coding agent [VERIFIED-fetched:https://raw.githubusercontent.com/google/agents-cli/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Committed JSON evalset + config-separated metrics | ⚙ Feature adoption | §11.3 verification suite format | Versioned recorded cases with metrics as configuration (not code) makes verification reproducible and reviewable — exactly the artifact LastHarness's verification dept should own per department. |
| `eval generate` / `eval grade` split | ⚙ Feature adoption | §11.3 replay pipeline | Separating trace capture from grading lets LastHarness re-score old evidence under new metrics and replay recorded sessions against candidate builds deterministically. |
| Local code-execution custom metrics (`evaluate(instance)` in-process) | ⚙ Feature adoption | §11.3 deterministic checks | Deterministic scoring functions running inside the local process fit the self-hosted boundary; ideal for machine-verifiable pass criteria alongside LLM judgment. |
| `final_response_match` golden-answer comparison + `hallucination` claim-vs-context check | 🧩 Mechanism only | Evidence grading tiers | Reference-based and grounding-based checks are two reusable verdict primitives for the verification suite's severity ladder. |
| `eval compare` / `eval analyze` failure clustering | 🧩 Mechanism only | Decision-log analytics | Before/after diffing and thematic failure clustering turn grade files into actionable regression reports feedable to the decision log. |
| Dataset synthesis via LLM user-simulator | 🧩 Mechanism only | Test-case bootstrapping | Cold-start coverage generator whose output remains a plain editable/committable evalset — simulation only at authoring time, replay stays deterministic. |
| `eval optimize` prompt auto-tuning loop | ❌ Conflict-skip | — | Hours-long autonomous mutation of agent prompts driven by cloud judge models; unreviewed self-modification conflicts with gated-change invariants. |
| Remote Vertex execution mode for custom metrics; deploy/publish to Cloud Run/GKE/Gemini Enterprise | ❌ Conflict-skip | — | Execution happens on Google Cloud — violates the self-hosted Linux boundary outright. Local-only subset is the adoptable surface. |

## Notable absences
No offline judge: every LLM-as-judge metric requires Gemini cloud models, so graded quality scores are not reproducible air-gapped (deterministic custom metrics are the only fully-local path); evalsets are ADK-shaped — no documented schema mapping to arbitrary agent frameworks; no signed/immutable storage of grade results (files are mutable artifacts); no per-department or role-scoped eval organization.
