# 02 · Memory & Context Engineering

Two clusters: **agent memory systems** (persistent knowledge across sessions — feeds LastHarness Information department design, §14.5/§6.3) and **context/token engineering** (compression, dedupe, routing — feeds Intelligence dept cost work, §14.4, and §7.1 token-discipline policy).

## Memory systems

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #5 | TokenGoat Mem | Local SQLite long-term memory for coding agents; trust levels, staleness anchors | Info,Dev | ok | high | - | https://github.com/DFKHelper/token-goat-mem |
| #14 | Claude-Mem | Persistent memory/compression layer for Claude Code sessions | Info,Dev | ok | high | https://claude-mem.ai | https://github.com/thedotmack/claude-mem |
| #19 | AI Memory | Cross-agent long-term markdown-wiki memory server with handoff injection | Info,Dev | ok | high | - | https://github.com/akitaonrails/ai-memory |
| #20 | Open Viking | Context database unifying agent memory, RAG, skills as virtual filesystem | Info | ok | high | https://openviking.ai | https://github.com/volcengine/OpenViking |
| #24 | TencentDB-Agent-Memory | Team-level memory hub turning conversations/docs/code into reusable agent assets | Info | ok | high | - | https://github.com/TencentCloud/tencentdb-agent-memory |
| #72 | Semantica | Knowledge-graph context infra with provenance and decision intelligence | Info,Res | ok | high | - | https://github.com/semantica-agi/semantica |
| #139 | CodeGraph | Pre-indexed code knowledge graph, auto-syncs on change, local-first, multi-agent aware | Info,Dev | ok | high | - | https://github.com/colbymchenry/codegraph |
| #140 | AgentMemory | Benchmark-topped persistent memory for AI coding agents | Info,Dev | ok | high | - | https://github.com/rohitg00/agentmemory |

## Context & token engineering

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #4 | TokenGoat | Hooks-based token optimizer: dedupes reads, compresses outputs, guards injection | Dev,Intel | ok | high | - | https://github.com/DFKHelper/token-goat |
| #11 | Context-Mode | MCP+hooks tool-output sandboxing, session continuity, context routing | Dev,Intel | ok | high | https://context-mode.com | https://github.com/mksglu/context-mode |
| #12 | Headroom CLI | Reversible context-compression proxy/library/MCP with cross-agent shared memory | Dev,Intel | ok | high | https://docs.headroomlabs.ai | https://github.com/headroomlabs-ai/headroom |
| #13 | SuperCompress | Query-aware context-compression API/proxy for agents and RAG | Dev,Intel | ok | high | https://www.supercompress.dev | https://github.com/Supercompress/Supercompress |

**Flags & notes**
- Crowded but distinct space: #4 hooks interception vs #11 MCP sandboxing vs #12 reversible proxy vs #13 query-aware API. None are dupes.
- Memory trio distinct: #5 fact store w/ staleness anchors vs #14 Claude Code plugin vs #19 cross-vendor wiki server.
- #20/#24 functionally overlap (both extract skills/wiki/code-graph from sessions) — relevant if Information dept compares context/memory backends; not dupes.
- Related-but-outside-this-file: #9 Caveman and #10 Ponytail (05-skills-plugins) attack the same token-waste problem from the skill side ("caveman shrinks what the agent says; ponytail shrinks what it builds").
