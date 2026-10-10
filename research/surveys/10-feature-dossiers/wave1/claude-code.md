# Claude Code — Feature Dossier
Registry: #48 | Category: 01 | Verified: 142,502 stars, no SPDX license (source-available repo, proprietary CLI), last push 2026-08-22

SLOT FOCUS: plan-approval checkpoint before write phase → §4.6 Project Agent gating.

## Feature inventory
- Plan mode (`claude --permission-mode plan`, `Shift+Tab` cycle, or `/plan` prompt prefix): Claude reads files and runs exploration commands but edits stay blocked until the user approves a plan; status bar shows `⏸ plan mode on` [VERIFIED-fetched:https://code.claude.com/docs/en/permission-modes]
- Plan approval flow with three explicit exits: "Yes, and use auto mode" (or "Yes, manually approve edits"), "No, keep planning"; approving exits plan mode and switches the session into the chosen write mode [VERIFIED-fetched:https://code.claude.com/docs/en/permission-modes]
- `Ctrl+G` opens the proposed plan in the default text editor for direct editing before execution proceeds; optional `showClearContextOnPlanAccept` approves-and-clears planning context in one step; accepted plans auto-title the session [VERIFIED-fetched:https://code.claude.com/docs/en/permission-modes]
- Six-mode permission ladder: `default` (manual), `acceptEdits`, `plan`, `auto`, `dontAsk` (exact allowlist for CI), `bypassPermissions`; mode settable per-session flag / project settings / managed org settings [VERIFIED-fetched:https://code.claude.com/docs/en/permission-modes]
- Deny rules block in *every* mode including `bypassPermissions`; writes to protected paths never auto-approved outside bypass; `rm`/`rmdir` on critical paths prompt even with allow rules [VERIFIED-fetched:https://code.claude.com/docs/en/permission-modes]
- `acceptEdits` auto-approval scoped to working directory + `additionalDirectories`; quoted-argument PowerShell commands still prompt because static validation fails — conservative fallback on ambiguity [VERIFIED-fetched:https://code.claude.com/docs/en/permission-modes]
- Auto mode: separate classifier model reviews each action before it runs, blocking escalation/destruction/exfil categories by default (curl|bash, force push, secret-bearing commits, transcript writes, disabling CI checks…); cached verdicts dropped on mode change [VERIFIED-fetched:https://code.claude.com/docs/en/permission-modes]
- Parallel isolated sessions via `claude --worktree <name>` git worktrees; headless `-p` piping for CI with `--allowedTools` allowlists and `dontAsk` mode [VERIFIED-fetched:https://code.claude.com/docs/en/common-workflows]
- Session resume (`--continue`/`--resume`/`/resume` picker) and PR-linked session retrieval (`claude --from-pr 1234`) [VERIFIED-fetched:https://code.claude.com/docs/en/common-workflows]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Plan mode: read-only research phase → presented plan → approval gates all writes | ⚙ Feature adoption | §4.6 Project Agent gating | Exactly the human-checkpoint-before-mutation shape LastHarness needs between plan and execute phases. The gate is structural (edits blocked by mode), not prompt-based — model cannot talk its way past it. |
| Edit-the-plan-in-editor before proceeding (`Ctrl+G`) | 🧩 Mechanism only | §4.6 plan artifact handling | Plan becomes an editable artifact the approver can redline instead of binary accept/reject; cheap to replicate over any plan document. |
| "No, keep planning" loop with feedback | 🧩 Mechanism only | §4.6 gating UX | Approval is a conversation, not a one-shot modal; iterate until plan is right, then flip to write phase. |
| Deny rules enforced in every mode incl. bypass; protected paths | ⚙ Feature adoption | Deny-by-default allow-list invariant | Confirms deny-overrides-everything layering: modes widen convenience, denies cap risk absolutely. Direct precedent for kernel-level hard caps above agent-visible permissions. |
| `acceptEdits` path-scoped auto-approval + prompt-on-unvalidatable-args | 🧩 Mechanism only | Write-scope policy | Scope-bounded auto-approve with fail-closed behavior when static analysis can't prove safety — matches LastHarness's deny-by-default posture. |
| Auto-mode classifier reviewing every action | ❌ Conflict-skip | — | Classifier is a second cloud model call per action (requires Opus/Sonnet 4.6+ via Anthropic API); violates local-first inference and adds third-party dependency inside the enforcement path. |
| Cloud Routines / scheduled tasks (Anthropic-managed cloud) | ❌ Conflict-skip | — | Execution happens off-machine at Anthropic — violates self-hosted Linux boundary outright. |
| `dontAsk` exact-allowlist headless CI mode | ⚙ Feature adoption | §11.x unattended run profile | For non-interactive runs: pre-approved tool list only, everything else denied without prompting — the cleanest published expression of deny-by-default for batch agents. |

## Notable absences
Core CLI is closed-source (repo carries plugins/docs only, no license); no programmatic plan-gate API documented — approval is interactive UI, so kernel-mediated gating needs a reimplementation rather than an embed; plan content itself is not persisted as a signed/versioned artifact tied to subsequent diffs; permission state lives in local settings files, not an immutable decision log; worktree sessions are standing processes, not ephemeral workers.
