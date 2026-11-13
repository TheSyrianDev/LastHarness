# Steal plan (proposal)

Status: PROPOSAL. Not approved by Omar yet. Planning only, no code. Each item lists what it is, where it would live in our crates, a rough size (S, M, L), dependencies, and how to test it.

## What was read

- `can1357/oh-my-pi` (README, package list): verified the README names "Hashline: edit by content hash" (anchors instead of retyped lines, stale anchors rejected) and an `edit` tool built on it; it lists `@oh-my-pi/snapcompact` as a dependency (package.json). I did not read the snapcompact source, so its method is unverified.
- `nousresearch/hermes-agent` (README, tree): verified the README claims agent-curated memory with periodic nudges, autonomous skill creation after complex tasks, skills that self-improve during use, FTS5 session search with LLM summaries, and agentskills.io compatibility. The tree has `MEMORY.md` / `USER.md` references, `gateway/` (WhatsApp identity etc.), `cron/`, and `skills/`. I did not read the code paths in detail.
- `0xsline/awesome-deepseek-harness`: a catalogue of plugins. Only the catalogue was read, not the plugins. Useful as a list of prior art, not as a source of design.

## NOW

### 1. Line-anchored (hashline) edits for small models
- What: the edit tool takes anchors (a short hash of each line) instead of exact old text. A stale anchor is rejected, so a small model cannot corrupt a file with a bad string match.
- Where: `crates/lh-tools/src/diff.rs` (edit application), `crates/lh-tools/src/tools.rs` (tool schema), `crates/lh-tools/src/runner.rs` (dispatch).
- Size: M.
- Depends on: nothing new.
- Test: unit tests on temp files: anchor match applies; stale anchor refused with file unchanged; whitespace-only differences still anchor. Live check with the test model (`LFM3-1.7B-balanced`) only when the machine is free.

### 2. Context compression
- What: when a session nears its context limit, older turns become a summary and the recent turns stay verbatim. The method used by snapcompact is not verified; the design here is ours.
- Where: `crates/lh-tools/src/compact.rs` already exists (extend it); `crates/lh-core/src/session.rs` (when to compact); `crates/lh-core/src/kv_cache.rs` (keep the prefix cache stable after a compact).
- Size: M.
- Depends: item 1 is independent; compaction should record what it dropped so memory (item 3) can keep facts.
- Test: a fixture transcript over the limit compacts to under the limit, keeps the last N turns verbatim, and keeps the system prompt. Mocked model returns a fixed summary.

## NOW (continued)

### 3. Memory: facts, preferences, search over past sessions, agent-curated
- What: `MEMORY.md` for facts about the work, `USER.md` for the person's preferences. The agent proposes edits (curated), and a search runs over past sessions.
- Where: `crates/lh-memory/src/conversations.rs` (past sessions, search), `crates/lh-memory/src/proposals.rs` (agent-proposed edits waiting for approval), `crates/lh-memory/src/page.rs` (files), `crates/lh-memory/src/maintenance.rs` (periodic nudges).
- Size: M-L.
- Depends: the existing memory approval flow; item 2 for what to keep.
- Test: proposals are not applied without approval; search returns a known past message; nudge fires after N turns in a fake clock test.

### 4. Self-written skills (agentskills.io format), saved after hard tasks, per department worker
- What: after a task that took many steps, the worker writes a skill file in the agentskills.io format. The skill is a proposal until the user approves it.
- Where: `crates/lh-learn/src/skills.rs` (exists), `crates/lh-learn/src/proposals.rs`, `crates/lh-team/src` (worker hooks).
- Size: M.
- Depends: item 3 (proposals flow).
- Test: a task over the step threshold yields one proposed skill file that validates against the format; nothing is written into the approved skill folder before approval.

## NEXT

### 5. Messaging gateway (Telegram, WhatsApp, Signal, Discord, email) into the same agent
- Where: `crates/lh-channels` (exists: Telegram, Matrix, Slack, Signal today). Add Discord and email adapters beside them.
- Size: L (each channel is its own adapter).
- Depends: channel auth in `crates/lh-secrets`.
- Test: adapter parses a recorded webhook payload and routes it to the same session as the GUI; no network in tests.

### 6. Scheduled jobs in plain words
- Where: `crates/lh-core/src/schedules_api.rs` (exists), a parser in `crates/lh-core` for phrases like "every weekday at 9".
- Size: S-M.
- Test: phrase table test; next-run times computed with a fixed clock.

### 7. Outside agents (Claude Code, Codex) as department workers
- Where: `crates/lh-team/src`, `crates/lh-provider` (a worker that runs an external CLI in a sandboxed folder).
- Size: L.
- Depends: permissions (`crates/lh-tools/src/perm.rs`).
- Test: a fake CLI script returns a canned answer; the worker reports it; permissions asked before any write.

### 8. Add-on hub
- Size: L. Depends on 7. Test: install and remove a sample add-on in a temp folder.

## LATER

- Browser relay: `crates/lh-pageview` exists; size L.
- LSP and debugger for the coding department: `crates/lh-tools/src/lsp.rs` exists; size L.
- Import from other agents (Claude Code, Codex, Hermes memories and skills): `crates/lh-core/src/export_api.rs` is the place to start; size M.
- Profiles: size M; depends on settings (`crates/lh-config`).

## Open questions for Omar

- Is the order above right, in particular items 1 and 2 first?
- Does the Hermes-style memory fit with the approval rules already in `.grill/`?
- Which messaging channels come first after item 4?

## Omar's answers (recorded 2026-11-13)

- Order approved as written: NOW 1-4, NEXT 5-8, LATER.
- Memory writes need Omar's approval, per the .grill rules.
- Telegram comes first among messaging channels (item 5).
- Coding of the plan has not started. Hermes additions come from the CEO.

## Additions from CEO relay (2026-11-13)

Source status: these items come from the CEO relay, not from reading the sources. Marked UNVERIFIED until someone reads the code. Hermes items were partly checked in its README only (memory, skills, FTS5 search, agentskills.io); the rest is unverified.

Order: hashline edits, context compression, snapshots and rollback, toolsets, memory, subagents, skills; then the small items; Telegram first for messaging. Coding waits for Omar's word ("don't start coding the plan yet").

### NOW additions
- Snapshots before file changes with `/rollback` (Hermes). Priority high. Size M. Where: `crates/lh-tools/src/runner.rs`, `crates/lh-tools/src/diff.rs`. Test: a write creates a snapshot; `/rollback` restores the file byte for byte. UNVERIFIED.
- Toolsets per chat or channel (web, terminal, files, memory) (Hermes). Size M. Where: `crates/lh-tools/src/tools.rs`, `crates/lh-tools/src/perm.rs`. Test: a chat with only "files" cannot call terminal. UNVERIFIED.
- Subagents with their own limited toolset and terminal, at most 3 running at once, as Department workers (Hermes). Size L. Where: `crates/lh-team/src`. Depends on toolsets. Test: a fourth subagent waits until one ends. UNVERIFIED.
- Context compression trigger at 50% of the window (Hermes). Size S. Where: `crates/lh-core/src/session.rs`. Extends item 2. Test: a fixture at 49% does not compact; at 51% it does. UNVERIFIED.

### Small items
- Auto-load AGENTS.md, CLAUDE.md and .cursorrules (Hermes). Size S. Where: `crates/lh-core/src/session.rs` or the prompt assembly. Test: a folder with each file puts its text in the prompt. UNVERIFIED.
- `@` references for file, folder, git diff and URL (Hermes). Size S-M. Where: `crates/lh-gui` input and `crates/lh-tools`. Test: each reference type expands to the expected text. UNVERIFIED.
- `/usage` and `/insights` (Hermes). Size S. Where: `crates/lh-spend` (usage), `crates/lh-memory` (insights). Test: fixture spend log gives the expected totals. UNVERIFIED.

### Ideas to consider later (from other agents)
- Per-task token budgets, live plan checklist, one-line install (Mercury). Size M. Where: `crates/lh-core` (budget), `crates/lh-gui` (checklist). UNVERIFIED.
- Last Engine one-click plug-in for Pi, Hermes and Codex (Magnitude). Size L. Depends on item 7 (outside agents). UNVERIFIED.

## Open
- Omar to confirm the order above before any coding.

## Compression backlog (CEO order, 2026-11-13)

Follow-ups to step 2 (context compression, `crates/lh-tools/src/compact.rs`). Not started.

- (a) Prune old tool outputs to one-line stubs before summarizing. Where: `compact.rs` before the summary request. Size S. Test: a tool result older than the last turns becomes one line; newer results stay whole.
- (b) Protect the system prompt, the first user message and the last N turns from pruning and summary. Where: `compact.rs` (`keep_last_turns` exists; first user message and system prompt are not yet protected by name). Size S. Test: after compaction the system text, first user message and last N turns are byte-identical.
- (c) A long tool-heavy session test that stays under the window and keeps the task goal. Where: `crates/lh-tools/tests` with a mocked model. Size M. Test: 200 tool calls with a fake model; the estimate stays under the window after compaction; the first user message (task goal) is still present.

## Channel toolsets (for the messaging step)

Not wired yet. Channels (Telegram first) must pass a toolset limit the same way chats do: `toolsets` on channel creation, stored with the channel, applied last in `new_session`. Default for a channel: `memory` only until Omar says otherwise. Test: a channel created with `["web"]` offers only web tools, also after a restart.

## Build speed: revisit later

- revisit later: Blaze needs nightly + blaze-releases.ix.dev, claims 11x avg / up to 92x on edit rebuilds. Skipped by Omar; not installed.
