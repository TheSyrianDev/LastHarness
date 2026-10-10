import io

def patch(path, pairs):
    c = io.open(path, encoding='utf-8').read()
    for old, new in pairs:
        assert old in c, f"MISSING {path}: {old[:70]}"
        c = c.replace(old, new, 1)
    io.open(path, 'w', encoding='utf-8', newline='').write(c)

# --- 06: #137 #138 #143 ---
patch('research/surveys/06-orchestration.md', [(
 '| #136 | Agent Zero | Autonomous agentic framework running on its own Docker computer w/ browser+terminal+files | Dev,Res | ok | high | https://www.agent-zero.ai | https://github.com/agent0ai/agent-zero |',
 '''| #136 | Agent Zero | Autonomous agentic framework running on its own Docker computer w/ browser+terminal+files | Dev,Res | ok | high | https://www.agent-zero.ai | https://github.com/agent0ai/agent-zero |
| #137 | Hermes Studio | Web dashboard for Hermes Agent (#53): sessions, scheduled jobs, multi-platform chat | Product,Kernel | ok | high | - | https://github.com/EKKOLearnAI/hermes-studio |
| #138 | LoopX | Provider-neutral long-horizon control plane: durable goals, evidence logs, governed handoffs | Dev,Res | ok | high | - | https://github.com/huangruiteng/loopx |
| #143 | Ruflo | Original agent meta-harness: deploy multi-player swarms coordinating autonomous workflows | Dev,Res | ok | high | https://ruflo.dev | https://github.com/ruvnet/ruflo |''')])

# --- 02: #139 #140 ---
patch('research/surveys/02-memory-context.md', [(
 '| #72 | Semantica | Knowledge-graph context infra with provenance and decision intelligence | Info,Res | ok | high | - | https://github.com/semantica-agi/semantica |',
 '''| #72 | Semantica | Knowledge-graph context infra with provenance and decision intelligence | Info,Res | ok | high | - | https://github.com/semantica-agi/semantica |
| #139 | CodeGraph | Pre-indexed code knowledge graph, auto-syncs on changes, local-first; Hermes/Claude/Codex-aware | Info,Dev | ok | high | - | https://github.com/colbymchenry/codegraph |
| #140 | AgentMemory | Benchmark-topped persistent memory for AI coding agents | Info,Dev | ok | high | - | https://github.com/rohitg00/agentmemory |''')])

# --- 07: #141 #142 ---
patch('research/surveys/07-media-apps.md', [(
 '| #133 | Hunyuan3D | Text/image-to-3D generation family: assets, PBR materials, agentic scene generation | Product,Res | ok | high | - | https://github.com/Tencent-Hunyuan/Hunyuan3D-2 |',
 '''| #133 | Hunyuan3D | Text/image-to-3D generation family: assets, PBR materials, agentic scene generation | Product,Res | ok | high | - | https://github.com/Tencent-Hunyuan/Hunyuan3D-2 |
| #141 | QwenPaw | Self-hosted personal AI assistant (Qwen ecosystem); own-machine or cloud deploy | Product,Auto | ok | high | - | https://github.com/agentscope-ai/QwenPaw |
| #142 | AstrBot | Multi-IM agent gateway + dev framework across LLMs and chat platforms (desktop sibling app) | Product,Auto | ok | high | https://astrbot.app | https://github.com/AstrBotDevs/AstrBot |''')])

# --- 04: #144 ---
patch('research/surveys/04-local-inference.md', [(
 '| #134 | Ox Alpha | Stealth frontier reasoning model via OpenRouter/Zen routes; anonymous provider | Intel | no-repo(closed) | high | https://oxalpha.io | https://github.com/ox-alpha-installer/ox-alpha-installer |',
 '''| #134 | Ox Alpha | Stealth frontier reasoning model via OpenRouter/Zen routes; anonymous provider | Intel | no-repo(closed) | high | https://oxalpha.io | https://github.com/ox-alpha-installer/ox-alpha-installer |
| #144 | 9router | Free-token router with auto-fallback across providers for coding-agent CLIs | Intel | ok | high | - | https://github.com/decolua/9router |''')])

# --- 01: #145 #146 (insert before Flags section) ---
p = 'research/surveys/01-coding-agents.md'
c = io.open(p, encoding='utf-8').read()
fa = '**Flags & notes**'
assert fa in c
add = '''| #145 | cmux | Ghostty-based macOS terminal with vertical tabs + notifications purpose-built for coding agents | Dev | license-null | high | - | https://github.com/manaflow-ai/cmux |
| #146 | Qwen Code | Alibaba's open-source terminal AI coding agent (Apache-2.0) | Dev | ok | high | https://github.com/QwenLM/qwen-code | https://github.com/QwenLM/qwen-code |

'''
c = c.replace(fa, add + fa, 1)
io.open(p, 'w', encoding='utf-8', newline='').write(c)

# --- 09 ledger rows ---
p = 'research/extraction/09-feature-extraction.md'
c = io.open(p, encoding='utf-8').read()
anchor = '| #136 | Agent Zero | conflict | Docker-hosted computer execution violates \u00a716.1 no-third-party-runtimes; tool-creation + persistence patterns extractable |'
assert anchor in c
rows = [
 '| #137 | Hermes Studio | adopt | Owner-console prior art for proposal #1 \u2014 session/job/chat renders over an existing agent\'s state |',
 '| #138 | LoopX | adopt | Strongest R11 handoff-packet prior art: durable goals + governed handoffs + evidence logs as first-class artifacts |',
 '| #139 | CodeGraph | adopt | Local-first pre-indexed code KG with change-sync \u2014 upgrades \u00a76.3 code_index/repo-map family |',
 '| #140 | AgentMemory | note | Benchmark-topped persistent memory \u2014 compare against adopted staleness-anchor design (#5) before any swap |',
 '| #141 | QwenPaw | skip | Odysseus/holaOS competitive cluster; surfaces exist as \u00a74.5/\u00a714.5/\u00a714.6 |',
 '| #142 | AstrBot | conflict | Multi-IM gateway = standing channel listeners (\u00a716.4/\u00a716.5); adapter shape already deferred post-V1 |',
 '| #143 | Ruflo | conflict | Standing multi-player swarm coordination violates \u00a716.5/\u00a74.4; swarm-coordination patterns extractable as design notes |',
 '| #144 | 9router | conflict | Quota-evasion token routing \u2014 same class as #62 ruling; Path-B singular-endpoint rule holds |',
 '| #145 | cmux | note | License-null (flagged); terminal UX for agent sessions = owner-console render evidence (R7) |',
 '| #146 | Qwen Code | skip | Competing terminal coding agent; SDK-lock (\u00a716.2) makes it reference-only |',
]
c = c.replace(anchor, anchor + '\n' + '\n'.join(rows), 1)
c = c.replace('*Ledger ends \u2014 135 rows.', '*Ledger ends \u2014 145 rows.', 1)
io.open(p, 'w', encoding='utf-8', newline='').write(c)

# --- Index counts + header ---
patch('research/surveys/00-index.md', [
 ('| [01-coding-agents.md](01-coding-agents.md) | Coding agents & harnesses (terminal/IDE/cloud/companions) | 31 |',
  '| [01-coding-agents.md](01-coding-agents.md) | Coding agents & harnesses (terminal/IDE/cloud/companions) | 33 |'),
 ('| [02-memory-context.md](02-memory-context.md) | Memory systems & context/token engineering | 10 |',
  '| [02-memory-context.md](02-memory-context.md) | Memory systems & context/token engineering | 12 |'),
 ('| [04-local-inference.md](04-local-inference.md) | Local inference runtimes & model tooling | 10 |',
  '| [04-local-inference.md](04-local-inference.md) | Local inference runtimes & model tooling | 11 |'),
 ('| [06-orchestration.md](06-orchestration.md) | Orchestration frameworks, workflow engines, observability | 18 |',
  '| [06-orchestration.md](06-orchestration.md) | Orchestration frameworks, workflow engines, observability | 21 |'),
 ('| [07-media-apps.md](07-media-apps.md) | Media gen/editing, office suites, end-user AI apps | 17 |',
  '| [07-media-apps.md](07-media-apps.md) | Media gen/editing, office suites, end-user AI apps | 20 |'),
 ('**130 active entries + 2 merges + 2 delists**',
  '**142 active entries + 2 merges + 2 delists**'),
])

# --- Verifier C3 + C7 ---
p = '.omo/ulw-research/20260822-143607/verify-research.ps1'
v = io.open(p, encoding='utf-8').read()
pairs = [
 ('    $expected = @(1..136 | Where-Object { $_ -ne 45 -and $_ -ne 38 })',
  '    $expected = @(1..146 | Where-Object { $_ -ne 45 -and $_ -ne 38 })'),
 ('-eq 135) { Pass "C3 extraction coverage: all 135 ledger entries present (132 active + merged/delisted history)" }',
  '-eq 145) { Pass "C3 extraction coverage: all 145 ledger entries present (142 active + merged/delisted history)" }'),
 ('Fail "C3 coverage: $($fxNums.Count)/135 rows',
  'Fail "C3 coverage: $($fxNums.Count)/145 rows'),
]
for o, n in pairs:
    assert o in v, o[:60]
    v = v.replace(o, n, 1)
L = []
L.append('# --- C7: wave-4 dossier coverage (10 dossiers) ---')
L.append('$dman4 = Join-Path $Root ".omo/ulw-research/20260824-100508/wave4-manifest.txt"')
L.append('if (Test-Path $dman4) {')
L.append('    $ok4=0; $bad4=@()')
L.append('    Get-Content $dman4 | ForEach-Object {')
L.append('        $f = Join-Path $Root ("research/surveys/10-feature-dossiers/wave4/" + $_ + ".md")')
L.append("        if ((Test-Path $f) -and (Select-String -Path $f -Pattern 'Feature Dossier' -Quiet) -and (Select-String -Path $f -Pattern 'Per-feature verdict' -Quiet)) { $ok4++ } else { $bad4 += $_ }")
L.append('    }')
L.append('    if ($ok4 -eq 10 -and $bad4.Count -eq 0) { Pass "C7 wave-4 dossier coverage: all 10 present" }')
L.append('    else { Fail ("C7 wave-4 dossiers: " + $ok4 + "/10 ok; bad: " + ($bad4 -join ", ")) }')
L.append('} else { Fail ("wave-4 manifest missing: " + $dman4) }')
L.append('')
L.append('Write-Output "---"')
v = v.replace('Write-Output "---"', '\n'.join(L), 1)
io.open(p, 'w', encoding='utf-8', newline='').write(v)

with open('.omo/ulw-research/20260824-100508/wave4-manifest.txt', 'w') as f:
    f.write('\n'.join(['hermes-studio','loopx','codegraph','agentmemory','qwenpaw','astrbot','ruflo','9router','cmux','qwen-code']) + '\n')

# --- Corrections ---
c = io.open('research/extraction/99-corrections.md', encoding='utf-8').read()
block = '''

## Wave-5 trending registrations (2026-08-25)

| Entry | Note |
|---|---|
| #137\u2013#146 | Ten GitHub-trending finds registered after API verification (stars/license/pushed all fresh). License flags: #137 NOASSERTION, #142 AGPL-3.0 (self-host OK / embed restrictive), #145 license-NULL. |
| #53 stat refresh | Hermes Agent now 236,302\u2605 (API 2026-08-25); genuine-growth trajectory confirmed. |
| #146 decision | QwenLM/qwen-code registered as distinct product (Alibaba terminal agent, Apache-2.0, 27.4k\u2605) \u2014 not adjacent to any registered Qwen ecosystem entry (MiMo Code #51 is Xiaomi). |
| Tier-B pre-dispositions | #143 ruflo + #144 9router registered WITH conflict expectations per triage; dossiers carry full teardown treatment. |
'''
assert 'Wave-5 trending' not in c
io.open('research/extraction/99-corrections.md', 'w', encoding='utf-8', newline='').write(c + block)
print('ALL DONE')
