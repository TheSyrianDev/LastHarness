import io

# 1. Blueprints: proposal #1 amendment + R11 strengthening + wave-5 note
p = 'research/extraction/11-department-blueprints.md'
c = io.open(p, encoding='utf-8').read()
appendix = '''

---

# Wave 5 findings (trending registrations)

> Source: 10 dossiers in [`10-feature-dossiers/wave4/`](10-feature-dossiers/wave4/). No new gap rows \u2014 this wave's value is *prior-art depth* on existing rows and proposals.

## Cross-reference amendments
- **Proposal #1 Owner Console** \u2014 **#137 Hermes Studio** is the strongest single prior art found to date: a shipped web dashboard rendering another agent's sessions, scheduled jobs, and multi-platform chat with frozen run snapshots and edge-decision evidence playback. It demonstrates the console as a product category over a foreign runtime \u2014 exactly \u00a716.10.1's shape. **#145 cmux** adds the terminal-side render (per-session sidebar, attention triage) as R7 evidence.
- **R11 Handoff packets** \u2014 **#138 LoopX** is the strongest prior art yet: typed handoff artifacts (claims/leases/task boundaries/typed continuation), durable goals surviving provider changes across Codex/Claude Code, quota-gated ticks (spend appended only after validated writeback), and audited safe-fallback structurally barred from bypassing gates. Its governed-handoff artifact set should be read alongside the adopted #59 save-gated rounds when drafting the \u00a716.6 input schema.
- **Freshness completes both halves** \u2014 adopted #5 staleness anchors cover write-time anchoring; **#139 CodeGraph** contributes read-time enforcement (per-file staleness banner, connect-time hash catch-up, never answer stale silently). Together they specify \u00a76.3 freshness end-to-end.
- **Provenance tagging confirmed cross-tool**: codegraph `synthesizedBy` edges, loopx evidence-strength labels, ruflo signed witnesses \u2014 trust-tier metadata in \u00a76.3 is market-standard, not bespoke.
- **Standing-process conflict cluster re-confirmed 4\u00d7 in one wave** (astrbot channels, qwenpaw heartbeats, ruflo queen/workers/federation, qwen-code daemon/IM bots) \u2014 the litmus keeps settling these without spec changes.

## License-integrity notes (logged in 99-corrections.md)
#137 resolves to **BSL-1.1** (source-available \u2014 pattern-level prior art only, no code reuse); #145 README claims GPL-3.0-or-later while GitHub API reports NOASSERTION \u2014 null-license flag stands pending LICENSE-file inspection.
'''
assert 'Wave 5 findings' not in c
io.open(p, 'w', encoding='utf-8', newline='').write(c + appendix)

# 2. Corrections addendum
c = io.open('research/extraction/99-corrections.md', encoding='utf-8').read()
c += '''
| #137 / #145 license follow-up (dossier pass) | #137 hermes-studio NOASSERTION resolves to **BSL-1.1** (Business Source License, source-available): pattern-level prior art only \u2014 no code reuse. #145 cmux: README claims GPL-3.0-or-later but GitHub API reports NOASSERTION \u2014 flag stands until LICENSE file inspected in-repo. |
'''
io.open('research/extraction/99-corrections.md', 'w', encoding='utf-8', newline='').write(c)
print('blueprints + corrections amended')
