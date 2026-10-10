# 05 · Skills, Plugins & Agent Role Packs

Skill collections, plugin marketplaces, awesome lists, role packs, and prompt/guideline packs. Feeds LastHarness department acquisition paths (§5.3) and the `skills` memory category (§14.5 Procedural store).

## Skill & plugin collections

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #1 | GStack | Garry Tan's Claude Code skill pack: 23 specialist roles plus workflow tools | Dev | ok | high | - | https://github.com/garrytan/gstack |
| #2 | ECC (Everything Claude Code) | Claude Code plugin toolkit: agents, commands, skills, rules, hooks | Dev | ok | high | - | https://github.com/WorldFlowAI/everything-claude-code |
| #3 | Agency Agents | Curated subagent role packs for agency-style development work | Dev | ok | med | - | https://github.com/msitarzewski/agency-agents |
| #6 | Matt Pocock Skills | Matt Pocock's Claude Code skill collection (AI Hero) | Dev | ok | high | https://www.aihero.dev/skills | https://github.com/mattpocock/skills |
| #21 | Anthropic Cybersecurity Skills | 817 cybersecurity skills for AI agents, MITRE/NIST-mapped | Sec | ok | high | - | https://github.com/mukul975/Anthropic-Cybersecurity-Skills |
| #35 | Awesome Claude Plugins | Curated production-ready Claude Code plugin collection/marketplace | Dev | ok | high | - | https://github.com/composio-community/awesome-claude-plugins |
| #36 | Awesome Codex Skills | Curated practical Codex CLI skill collection | Dev | ok | high | - | https://github.com/composio-community/awesome-codex-skills |
| #39 | Browser Skills | Curated browser-automation skills: scraping, monitoring, web actions for agents | Res,Auto | ok | high | - | https://github.com/composio-community/browser-skills |
| #40 | OpenClaw Composio Plugin | Plugin bringing 1000+ Composio app integrations into OpenClaw | Dev,Auto | ok | high | - | https://github.com/composio-community/openclaw-composio-plugin |
| #41 | Support Skills | Customer-support skill packs: ticket triage, SLA tracking, reply drafting | Auto,Product | ok | high | - | https://github.com/composio-community/support-skills |
| #44 | oh-my-openagent | Agent/hook/skill plugin pack supercharging OpenCode and Codex harnesses | Dev | ok | high | - | https://github.com/code-yeongyu/oh-my-openagent |
| #68 | Oh-my-Hermes | Opinionated skill/workflow layer (36 skills, 7 agents) for Hermes Agent | Dev,Auto | ok | high | - | https://github.com/Salomondiei08/oh-my-hermes |
| #96 | Competitive Ads Extractor | Claude skill extracting/analyzing competitors' ads from ad libraries | Res | ok | high | https://buildwithclaude.com/skill/competitive-ads-extractor | https://github.com/ComposioHQ/awesome-claude-skills |
| #97 | Deep Research Skills | Two-phase outline+deep-research skill pack for Claude Code/OpenCode/Codex | Res | ok | high | - | https://github.com/Weizhena/deep-research-skills |
| #98 | Agent Skills | "Flow": Claude Code plugin monorepo (workflow/research/engineering/content packs) | Dev,Res,Info | ok | high | - | https://github.com/kzarzycki/agent-skills |
| #99 | ElevenLabs Skills | Official agent skills for ElevenLabs TTS/audio/music APIs | Product,Res | ok | med | https://elevenlabs.io/ | https://github.com/elevenlabs/skills |
| #108 | Google Skills | Official Google/GCP agent-skills collection plus harness plugins | Dev | ok | high | - | https://github.com/google/skills |
| #123 | Scientific Agent Skills | Scientific agent skills pack turning AI agents into research scientists | Res | ok | high | - | https://github.com/K-Dense-AI/scientific-agent-skills |
| #125 | ThreeUI | 160+ procedural Three.js hero/landing components copyable as agent prompts | Dev,Product | ok | high | https://threeui.com | https://github.com/MengTo/threeui |
| #128 | Qiaomu-seo | SEO agent skill auditing Google/Bing/AI-search with evidence-graded findings | Res | ok | high | - | https://github.com/joeseesun/qiaomu-seo |

## Design & engineering guideline packs

| # | Tool | Purpose | Dept-fit | Status | Conf | Official | GitHub |
|---|------|---------|----------|--------|------|----------|--------|
| #8 | Dictionary of AI Coding | Plain-English glossary of AI-coding jargon and concepts | Res,Dev | ok | high | - | https://github.com/mattpocock/dictionary-of-ai-coding |
| #9 | Caveman | Terse-output skill plus local compression proxy cutting agent tokens | Dev,Intel | ok | high | https://getcaveman.dev | https://github.com/JuliusBrussee/caveman |
| #10 | Ponytail | Skill/plugin forcing minimal-code YAGNI ladder into coding agents | Dev | ok | high | https://ponytail.dev | https://github.com/DietrichGebert/ponytail |
| #25 | Awesome Design.md | Curated DESIGN.md brand design-system files for AI UI generation | Dev,Product | ok | high | - | https://github.com/VoltAgent/awesome-design-md |
| #26 | Official Design.md | Index of first-party DESIGN.md files published by companies themselves | Dev,Product | ok | high | - | https://github.com/VoltAgent/official-design-md |
| #27 | Karpathy-Inspired Guidelines | CLAUDE.md guideline pack countering LLM coding pitfalls (Karpathy-derived) | Dev | ok | high | - | https://github.com/multica-ai/andrej-karpathy-skills |
| #28 | Emil Kowalski's Skills | UI animation/design skill pack for AI agents by Emil Kowalski | Dev,Product | ok | high | - | https://github.com/emilkowalski/skills |
| #132 | Impeccable | AI-harness design language: 23 commands, 59 detector rules, DESIGN.md-native | Dev,Product | ok (installed) | high | https://impeccable.style | https://github.com/pbakaus/impeccable |

**Flags & notes**
- **LICENSE-NULL pair (2026-08-24)**: #8 dictionary-of-ai-coding and #98 agent-skills-flow have NO license (GitHub API null) — verbatim adoption blocked until licensing resolves; content-reference only.
- **#45 oh-my-opencode merged into #44** (repo rename) — see 99-corrections.md.
- **#9 + #10 are a designed pair**, not rivals: "caveman shrinks what the agent says; ponytail shrinks what it builds."
- **#21**: community project by mukul975, explicitly NOT affiliated with Anthropic PBC despite the name.
- **#96**: verified to live inside ComposioHQ/awesome-claude-skills (skill-within-list-repo); link correct.
- **#97**: exposes exactly the `research-*` skill set already installed in this opencode environment (`~/.config/opencode/skills/research*`) — treat as existing dependency, not a new candidate.
- **#1 GStack** (129k★) ships GBrain persistent-memory integration + gstack-context-bill, making it adjacent to the 02-memory cluster despite being a skill pack.
- **#98**: zero-star personal monorepo — low adoption risk if skipped.
- **#25/#26**: companion VoltAgent repos (community-extracted collection vs first-party adoption tracker); related, not dupes.
- **#132 Impeccable (added 2026-08-25)**: 61k★+, Apache-2.0, npm `impeccable` v3.6.0, pushed daily. **Already installed in this OpenCode environment** (`~/.config/opencode/skills/impeccable/`, skill v4.0.1) — existing dependency like #97. Its DESIGN.md output is Google-Stitch-format-compatible with #25/#26. Notable: deterministic detector runs with NO LLM (CI-gate capable); hooks into every major harness incl. this one's OpenCode.
- **#132 vs #28 Emil Kowalski**: complementary — Emil's pack teaches motion/micro-interaction principles; Impeccable is a command vocabulary + deterministic anti-slop detector. Both can coexist in the skills registry.
