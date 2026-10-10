# Qwen Code — Feature Dossier
Registry: #146 | Category: 01 | Verified: 27,362★, Apache-2.0, pushed 2026-08-25

Skip-tier dossier per ledger: "Competing terminal coding agent; SDK-lock (§16.2) makes it reference-only" (09-feature-extraction:264). Ticket directive: inventory how it differs from its Gemini CLI lineage. It is a fork-and-diverge case: "originally based on Google Gemini CLI v0.8.2 … Starting from Qwen Code v0.1, we stopped syncing with upstream and began independent development as a multi-protocol, multi-platform agent framework" [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md].

## Feature inventory
- **Claude-Code-parity feature matrix (self-published comparison)**: SubAgents, Agent Teams, Dynamic Workflows, Auto-Memory, Auto-Skills, Hooks, built-in skills (/review /batch /loop /bugfix), MCP, Plan Mode, LSP Integration, Auto Mode, Sandbox, Git Worktrees, Computer Use, IDE plugins (VS Code/JetBrains/Zed), SDK, headless mode, session management — all claimed ✓ parity with Claude Code; improvement-report doc linked to a third-party repo [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md] [REPORTED-unverified for the parity claims themselves]
- **Multi-protocol provider layer (the main divergence from Gemini CLI)**: OpenAI/Anthropic/Gemini/Qwen API shapes plus any third-party provider and local models via Ollama/vLLM — switchable at runtime [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md]
- **Agent Arena**: multi-model head-to-head on the same task — listed as a Qwen-Code-only column vs Claude Code [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md]
- **Daemon mode `qwen serve`**: shared agent session over HTTP+SSE (ACP), multiple clients one agent, experimental [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md]
- **IM channels `qwen channel`**: Telegram/DingTalk/WeChat/Feishu bots from the coding agent [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md]
- **SDKs in three languages**: TypeScript, Python, Java packages driving the qwen executable [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md]
- **Desktop app + ecosystem GUIs**: official desktop releases plus third-party AionUi / Gemini-CLI-desktop UIs targeting it [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md]
- **Cross-agent delegation skill ("Qwen Code Claw")**: SKILL.md other agents fetch to install/authenticate and delegate coding tasks via acpx/ACP [VERIFIED-fetched:https://raw.githubusercontent.com/QwenLM/qwen-code/main/README.md]
- **Self-referential development claim**: "using its own agent and models to file issues, submit PRs, review code, and run tests" [REPORTED-unverified — dogfooding narrative]
- **Free OAuth tier discontinued**: Alibaba ended Qwen Code's free OAuth tier 2026-04-15 [REPORTED-unverified — sourced from wave4 9router README cross-reference, not from qwen-code docs]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Product as terminal coding agent | ❌ Conflict-skip CONFIRMED (reference-only) | §16.2 SDK-lock; competing-agent skip tier | Ledger confirmed: a vendored framework with its own model-family gravity is exactly what §16.2 excludes; LastHarness observes it as market evidence only. Execution-location litmus is irrelevant because nothing is executed — reference status. |
| Multi-protocol runtime model switching (incl. Ollama/vLLM) | 🧩 Confirmation only | Hardware-tiered model packs; local-first inference | Same doctrine independently shipped by a major lab; confirms rather than informs. |
| Agent Arena (same task → multiple models head-to-head) | 🧩 Design note | Verifier dept evaluation harness idea | Cheap, useful pattern for offline harness evaluation of candidate model packs on identical tasks — recordable as an Automation-dept scheduled benchmark re-check rendering into the notification queue. Nothing adopted now. |
| Daemon mode (`qwen serve`, multi-client shared session) | ❌ Conflict-skip | §16.5 standing process | Resident shared-session server is the excluded shape even before the product-level skip. |
| IM channel bots | ❌ Conflict-skip | §16.4 messengers out of V1 | Direct inbound-channel operation; post-V1 adapter conversation only. |
| Cross-agent delegation via fetched SKILL.md | ❌ Conflict-skip | Supply-chain discipline §14.5 | "Fetch a URL and follow install instructions" is precisely the ingestion path LastHarness's skills gate exists to scan/deny; noted as the anti-pattern it guards against. |

## Notable absences
No governance surface at README depth: no permission ladder detail beyond feature-name mentions (Plan Mode/Sandbox named, not specified), no audit log, no egress policy, no memory-governance mechanics behind Auto-Memory. Parity claims are self-scored against Claude Code without methodology links that verify at README level. The Gemini-lineage divergence inventory resolves cleanly: provider multiplicity, agent teams/Arena, daemon serving, IM bots, desktop+SDK spread are the additions; core terminal-agent loop remains lineage-shared. For LastHarness this entry's function is competitive cartography — it documents where the open-source coding-agent baseline has converged (teams, subagents, skills, MCP everywhere) and adds nothing actionable beyond the Arena evaluation note.
