# 06 · Orchestration & Frameworks

Multi-agent frameworks, workflow engines, LLM app frameworks, and observability. Context for LastHarness's §16.2 framework decisions (Claude Agent SDK chosen; Google ADK rejected) and kernel/manager patterns (§16.9).

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #7 | Sandcastle | TypeScript library orchestrating sandboxed coding agents (Docker/Podman/Vercel) | Dev | ok | high | - | https://github.com/mattpocock/sandcastle |
| #18 | Munder-Difflin | Electron multi-agent harness wrapping terminal CLIs as coordinated agent office | Dev | ok | high | - | https://github.com/chaitanyagiri/munder-difflin |
| #23 | VoltAgent | TypeScript framework for building and observing AI agents | Dev,Res | ok | high | https://voltagent.dev | https://github.com/VoltAgent/voltagent |
| #30 | Flowise | Drag-and-drop visual builder for LLM apps/workflows | Dev,Res,Auto | ok | high | https://flowiseai.com | https://github.com/FlowiseAI/Flowise |
| #58 | Catalyst | AI scientist: evolution-based theory discovery atop coding-agent harnesses | Res | ok | high | - | https://github.com/imbue-ai/catalyst |
| #63 | Deep Agents | LangChain library for deep agents: planning, filesystem, subagents | Dev,Res | ok | high | - | https://github.com/langchain-ai/deepagents |
| #64 | LangFuse | Open-source LLM observability: tracing, evals, prompt management | Dev,Res,Intel | ok | high | https://langfuse.com | https://github.com/langfuse/langfuse |
| #65 | LangChain | Framework for building LLM apps: chains, tools, RAG | Dev,Res | ok | high | https://langchain.com | https://github.com/langchain-ai/langchain |
| #66 | LangGraph | Graph-based stateful agent orchestration runtime from LangChain | Dev,Res | ok | high | https://langchain.com/langgraph | https://github.com/langchain-ai/langgraph |
| #73 | open-kritt | Self-hosted platform orchestrating AI agents to find and validate vulnerabilities | Sec,Dev | ok | med | - | https://github.com/Kritt-ai/open-kritt |
| #87 | Buzz | Block's Nostr-relay team workspace where humans and AI agents collaborate | Dev,Auto | ok | high | https://buzz.xyz | https://github.com/block/buzz |
| #88 | YC QM | YC's multiplayer org-wide agent harness for Slack/web work | Dev,Auto,Info | ok | high | https://qm.ycombinator.com | https://github.com/yc-software/qm |
| #102 | Vercel AI SDK | TypeScript toolkit/framework for building LLM-powered apps | Dev,Product | ok | high | https://ai-sdk.dev | https://github.com/vercel/ai |
| #107 | pi-rlm | Recursive Language Model decomposition extension+CLI for Pi coding agent | Dev | ok | high | - | https://github.com/manojlds/pi-rlm |
| #109 | Google Agents CLI | CLI+skills to scaffold/eval/deploy ADK agents on Google Cloud | Dev | ok | high | - | https://github.com/google/agents-cli |
| #113 | DeerFlow | ByteDance long-horizon SuperAgent harness: researches, codes, creates (v2 rebuild) | Dev,Res | ok | high | - | https://github.com/bytedance/deer-flow |
| #136 | Agent Zero | Autonomous agentic framework running on its own Docker computer w/ browser+terminal+files | Dev,Res | ok | high | https://www.agent-zero.ai | https://github.com/agent0ai/agent-zero |
| #137 | Hermes Studio | Web dashboard for Hermes Agent (#53): sessions, scheduled jobs, multi-platform chat | Product,Kernel | ok | high | - | https://github.com/EKKOLearnAI/hermes-studio |
| #138 | LoopX | Long-horizon control plane: durable goals, evidence logs, governed handoffs across agents | Dev,Res | ok | high | - | https://github.com/huangruiteng/loopx |
| #143 | Ruflo | Original agent meta-harness: deploy multi-player swarms coordinating autonomous workflows | Dev,Res | ok | high | https://ruflo.dev | https://github.com/ruvnet/ruflo |
| #114 | trycompai/crm | Self-hosted agent-first CRM: agent does research/data-entry via evidence ledger (MIT) | Dev,Res,Product | ok | high | - | https://github.com/trycompai/crm |
| #127 | holaOS | All-in-one AI agent workspace running Claude Code/Codex with shared memory | Product | ok | high | https://www.holaos.ai | https://github.com/holaboss-ai/holaOS |

**Flags & notes**
- **#136 Agent Zero**: executes inside Docker containers — container-hosted execution conflicts with §16.1 no-third-party-runtimes; framework patterns (tool creation, multi-agent cooperation, persistence) remain extractable as design notes.
- **#38 Composio Open Agents MERGED into #37 Vercel Open Agents** (2026-08-24): repo dissolved into stale fork duplicate; credential-vault mechanics moved to design-note (see 99-corrections). **#129 AgentReach DELISTED** (claimed product refuted; fallback repo stale). Both recoverable via ledger.
- **#58 Catalyst** runs atop Claude Code/Gemini CLI/Codex CLI; evolution workflows can cost ~$1000/workflow at API billing — Intel-relevant (§14.4).
- **#109**: README states it is "a tool *for* coding agents, not a coding agent itself" hence orchestration despite the name.
- **#87 Buzz ≠ chidiwilliams/buzz** (transcription app); **#79 Superset ≠ Apache Superset** (BI) — see 01-coding-agents.
- **#107 pi-rlm**: RLM decomposition relates to the Prime Agent REPL abstraction noted in §16.6 addenda (design note, not adopted).
