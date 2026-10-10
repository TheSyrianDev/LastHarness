# Conductor — Feature Dossier
THIN-SOURCE
Registry: #78 | Category: 01 | Verified: unverified — closed-source Mac app, no public repo; official site/docs only (conductor.build), current as of fetch

SLOT FOCUS: parallel-dispatch review board over live worktrees — owner-console gap.

## Feature inventory
- Runs Claude Code, Codex, Cursor, and OpenCode agents in parallel on the Mac, each task getting "its own workspace, branch, files, terminal, diff, and review path" [VERIFIED-fetched:https://conductor.build/docs]
- Workspace = unit of delegation, branch/PR = unit of integration: one workspace per shippable unit; create from a branch, pull request, GitHub issue, or Linear issue (`Cmd+Shift+N` / `...` menu) [VERIFIED-fetched:https://conductor.build/docs/concepts/workflow]
- Diff Viewer review board (`Cmd+Shift+D`): review changed files, leave inline comments, comments become composer attachments sent back to the agent that made the change; separate `Review` action for an agent-driven review alongside manual review [VERIFIED-fetched:https://conductor.build/docs/concepts/workflow]
- Checks tab aggregating pre-merge state: git status, CI runs, deployments, review comments, and todos before merge is allowed [VERIFIED-fetched:https://conductor.build/docs/concepts/workflow]
- PR flow (`Cmd+Shift+P`): draft PR description, respond to review comments, fix failing checks, follow GitHub Actions status checks for the branch, merge when green [VERIFIED-fetched:https://conductor.build/docs/concepts/workflow]
- Two dispatch topologies: multiple workspaces for independent streams vs multiple agents inside ONE workspace sharing branch/files (implementer + reviewer + test-repairer patterns); decision guide table [VERIFIED-fetched:https://conductor.build/docs/concepts/parallel-agents]
- Per-workspace app processes and terminal so each stream can run/test independently and reach review on its own schedule [VERIFIED-fetched:https://conductor.build/docs/concepts/workflow]
- Archive finished workspaces; restore later from History pane including chat history [VERIFIED-fetched:https://conductor.build/docs/concepts/workflow]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| One isolated worktree+branch per dispatched task | ⚙ Feature adoption | Owner-console parallel dispatch (owner-console gap) | Worktree isolation is the correct containment primitive: parallel agents never collide, and each stream's diff is exactly its contribution. Execution is local (user's machine). |
| Inline diff comments → attachments back to authoring agent | 🧩 Mechanism only | Owner-console review loop | The comment-roundtrip is the human-steers-agent channel inside a review board; portable pattern independent of Conductor's GUI. |
| Agent `Review` action + manual review split | 🧩 Mechanism only | §10.x verification | Machine pre-review filters noise before scarce human attention; matches LastHarness's verify-then-human-gate ordering. |
| Checks tab (git status + CI + comments + todos gate before merge) | ⚙ Feature adoption | Owner-console merge gate | Consolidated go/no-go board: nothing merges until all evidence channels are green — same aggregation LastHarness's console needs. |
| Multi-agents-in-one-workspace mode (reviewer shares live diff) | 🧩 Mechanism only | §16.x collaboration topology | Useful when shared context beats isolation; documents the tradeoff explicitly (same-file edit risk) rather than pretending fan-out is always right. |
| Issue → workspace → PR pipeline (GitHub/Linear intake) | ❌ Conflict-skip | — | Depends on external SaaS issue trackers + GitHub cloud; LastHarness's default-deny egress and self-hosted posture exclude it from adoption as-is. |
| Whole product as Mac-only closed-source GUI with standing agent sessions | ❌ Conflict-skip | — | macOS-only violates the Linux boundary; closed source prevents auditing where agent processes run; long-lived sessions conflict with ephemeral workers. Pattern extraction only. |

## Notable absences
No Linux build and no headless/API-first control plane documented on fetched pages (a `/docs/api` link exists but content was not retrievable — cannot verify kernel-embeddability); no role-based access or approval gating visible (single-owner tool); no immutable audit trail of who approved which merge; no memory layer at all; agents themselves are third-party runtimes (Claude Code etc.) whose execution boundaries Conductor does not constrain.
