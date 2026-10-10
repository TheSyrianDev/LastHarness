# pi-rlm — Feature Dossier
Registry: #107 | Category: 06 | Verified: 42 stars, no SPDX license detected via GitHub API, last push 2026-03-14

SLOT FOCUS: RLM chunked child delegation onto scoped children (§16.5) — decomposition/aggregation mechanics.

## Feature inventory
- Single registered `rlm` tool with lifecycle ops: `op=start` (default), `op=status` (run state or recent runs if id omitted), `op=wait` (bounded blocking), `op=cancel` [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]
- Depth-limited recursive decomposition: planner node decides `solve` vs `decompose`; child nodes recurse on subtasks; synthesizer node merges child outputs into the final result [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]
- Guardrail parameters on every run: `maxDepth` (recursion cap), `maxNodes` (total node budget, default 24), `maxBranching` (children per decomposition, default 3), `concurrency` (default 2), `timeoutMs`, and `mode` auto|solve|decompose forcing [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]
- Cycle detection by normalized task lineage — a decomposition that loops back to an ancestor task is rejected [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]
- Three child-execution backends: `sdk` (in-process Pi SDK sessions, no fresh CLI process per subcall — lowest overhead default), `cli` (fresh `pi -p` subprocess per subcall for isolation/debuggability), `tmux` (one detached tmux session per run with depth-oriented windows `depth-0`, `depth-1`, … and tiled panes; optional reuse of current session) [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]
- Child tool-scope profiles: `toolsProfile: "coding" | "read-only"` constrains what delegated children may do [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]
- Per-run artifacts written to `/tmp/pi-rlm-runs/<runId>/`: `events.jsonl` (event stream), `tree.json` (decomposition tree), `output.md` (final synthesis) [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]
- Standalone CLI wrapper (`pi-rlm --task ... --backend sdk --max-depth 3`) performing one synchronous `op=start`; `--live` renders the tree in real time by tailing `events.jsonl`; `--json` machine output [VERIFIED-fetched:https://raw.githubusercontent.com/manojlds/pi-rlm/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| planner → decompose/recurse → synthesizer pipeline | ⚙ Feature adoption | §16.5 chunked delegation mechanics | The exact three-role shape for splitting an oversized task onto scoped children and aggregating typed results. Children are ephemeral sessions spawned per subcall (sdk/cli backends) — satisfies the no-standing-worker invariant when torn down with the parent run. |
| maxDepth / maxNodes / maxBranching budgets + cycle detection on task lineage | ⚙ Feature adoption | §16.5 runaway containment | Hard numeric caps plus lineage-normalized cycle rejection are kernel-enforceable guardrails — decomposition can never outgrow its grant or recurse into itself. |
| `read-only` tools profile for children | ⚙ Feature adoption | Deny-by-default child scoping | Delegation narrows capability instead of inheriting it: children get a restricted allow-list, matching LastHarness's deny-by-default posture down the delegation chain. |
| `events.jsonl` + `tree.json` artifacts per run | 🧩 Mechanism only | Immutable decision log feed | Append-only event stream and explicit tree structure give the decision log a faithful record of who delegated what to whom and what came back. |
| `op=status/wait/cancel` supervision surface | 🧩 Mechanism only | Kernel-mediated run control | Parent/kernel observes and cancels without peer-to-peer messaging between children — control flows through the run owner only. |
| tmux backend (detached standing session per run, depth windows) | ❌ Conflict-skip | — | Standing tmux sessions and panes violate ephemeral-workers-only; observability value is already captured by events.jsonl tailing. |
| CLI wrapper as separate entrypoint | ❌ Conflict-skip | — | Second process surface duplicates what the kernel's own dispatch should own; LastHarness needs one mediated path. |

## Notable absences
No schema validation of child outputs (unlike omp's typed yields — synthesis consumes free-form results); no retry/backoff or partial-failure policy documented (a failed child's subtree handling is unspecified); no per-child memory scoping beyond the binary toolsProfile; no deduplication of sibling work; project is very small (42 stars), unlicensed, last pushed 2026-03 — treat as pattern source, not dependency.
