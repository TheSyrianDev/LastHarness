# Herdr — Feature Dossier
Registry: #81 | Category: 01 | Verified: 31,922★, Apache-2.0, last push 2026-08-24 (repo-verify.txt 2026-08-24; README lives on `master` branch — `main` 404s)

A Rust background server that owns the terminals of coding agents (Claude Code, Codex, Cursor, Opencode, Grok): sessions survive lid-close, network loss, and machine restart; agents drive it via CLI and socket API to spawn panes, prompt each other, and wait on one another's blocked state. Recorded conflict: §16.5 ephemeral dispatch; sanctioned path is §14.6 scheduled re-checks.

## Feature inventory
- Always-running background server: "herdr is a background server; the terminals live inside it" — agents keep working through lid-close/network-drop/machine-restart, and sessions come back for reattach from any terminal or over SSH [VERIFIED-fetched:https://raw.githubusercontent.com/herdrdev/herdr/master/README.md]
- Pane status marking: every pane marked working / blocked / idle; when an agent stops needing an answer, herdr announces it [VERIFIED-fetched:https://raw.githubusercontent.com/herdrdev/herdr/master/README.md]
- Agent-native control plane: agents spawn panes, prompt each other, and wait until another agent is genuinely blocked via CLI and socket API; dedicated agent-skill docs at herdr.dev [VERIFIED-fetched:https://raw.githubusercontent.com/herdrdev/herdr/master/README.md]
- Hosts existing agent CLIs without wrapping them — claude code, codex, cursor, opencode, grok — herdr "owns their terminals" [VERIFIED-fetched:https://raw.githubusercontent.com/herdrdev/herdr/master/README.md]
- Dual first-class input: tmux-style prefix keys plus click/drag/split mouse interaction [VERIFIED-fetched:https://raw.githubusercontent.com/herdrdev/herdr/master/README.md]
- Plugin system extending panes and workflows, with an external marketplace [REPORTED-unverified — README links the marketplace at herdr.dev/plugins; contents not fetched]
- Single Rust binary, no Electron; installs via curl script, Homebrew, or mise [VERIFIED-fetched:https://raw.githubusercontent.com/herdrdev/herdr/master/README.md]
- Session-state and remote-persistence documentation surfaces (herdr.dev/docs/session-state, /docs/persistence-remote) [REPORTED-unverified — linked from README, not fetched]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Always-running background runtime holding live agent sessions | ❌ Violates ephemeral dispatch (§16.5) | — | A resident server keeping agents "always-on" is precisely the standing-process form §16.5 excludes; the spec's own addendum (Prime Agent, 2026-08-12) rules the identical feature a conflict |
| Sessions survive restart + reattach anywhere (incl. SSH) | ❌ Violates ephemeral dispatch (§16.5) + worker-state rule | — | Continuity must come from stored goal state rehydrated per dispatch (decision log + memory, §12/§6), not from a kept-alive process; SSH reattach also widens remote access §13.4.2 forbids by default |
| Agent-to-agent prompting and block-waiting | ❌ Violates kernel mediation (§4.4) + peer-to-peer exclusion (§16.5) | — | Workers deciding among themselves with no central policy authority is the exact pattern §16.5's addendum names; all cross-agent traffic routes kernel-mediated |
| Working/blocked/idle pane-status triad | ⚙ Capability owner console owns | §16.6 Output 1 terminal status → console | The genuinely extractable gem: per-dispatch terminal states rendered live to the owner is presentation over data the dispatch contract already produces |
| Hosting foreign CLIs in owned terminals | ❌ Violates SDK lock (§16.2) | — | Cursor/Opencode/Codex as hosted engines are excluded substrates; LastHarness workers are Claude Agent SDK subagents only |
| Keyboard/mouse terminal UX | ⚙ Capability owner console owns | Owner-console interaction design note | Presentation-layer affordance only; no architectural weight either way |
| Plugin/marketplace extensibility | ⚙ Capability skills registry owns | §14.5 vetted versioned intake | Extensions enter through harness validation and versioned registration, never as arbitrary runtime plugins into a resident process |
| One-binary no-Electron packaging | 🧩 Pure architecture pattern | §16.1/§17.2 deployment hygiene | Minimal-footprint single-artifact deployment matches LastHarness's no-third-party-software-presenting-as-separate-app principle |

## Compliant re-implementation sketch
- Extract the observability value, not the runtime: owner console renders each active dispatch as working/blocked/idle straight from §16.6 Output 1 statuses and per-call traces (§16.6 Output 3) — herdr's clearest idea arrives free, since LastHarness's contract already emits this signal.
- "Keep long work moving" takes the spec-sanctioned path verbatim: Automation-department scheduled re-checks (§14.6) against stored goal state — the worker terminates, its goal and progress persist in the decision log/memory, and the next scheduled dispatch resumes rather than a process surviving.
- Blocked-on-human moments become owner-console notifications via the Automation notification queue (§14.6), answered by initiating a fresh dispatch with full context rehydration.
- Cross-agent handoffs route through the kernel (§4.4): a worker requests routing; it never prompts a peer or waits on one directly.
- Session transcript recovery after interruption comes from append-only dispatch records (§12), which are already crash-safe by construction.
- Stay out: the resident background server, terminal ownership of foreign CLIs, SSH reattach, socket-API peer messaging, and the plugin marketplace as a runtime extension point (skills registry covers vetted extension instead).

## Notable absences
No security model documented at README level — a socket API that lets any local agent spawn panes and prompt others has no auth story in the README; no egress control over what hosted agents do; no audit trail beyond terminal scrollback. Multi-agent coordination is presented purely as convenience, with no policy layer deciding *which* agent may wake which. Deeper docs exist at herdr.dev but were not fetched (README-level verification only).
