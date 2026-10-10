# LastHarness AI — Tool Research Registry

> Reorganized 2026-08-22 from the flat Freebuff dump (109 rows, archived verbatim at [`freebuff-dump-2026-08-20.md`](freebuff-dump-2026-08-20.md)). Link-level verification of the underlying data dates to 2026-08-20; classification added 2026-08-22; entries #110–#117 verified from the Instagram harvest (`harvest-instagram-2026-08-22.md`) on 2026-08-22; waves 2–3 of the same harvest registered as #118–#130 later that day (see `../99-corrections.md` → harvest-maintenance sections).

**143 active entries + 2 merges + 2 delists** (`../99-corrections.md`), organized by functional category, each tagged with the LastHarness department(s) it serves ([docs/archive/lastharness-v1-spec.md](../../docs/archive/lastharness-v1-spec.md)). *Count corrected 2026-08-31 — previously 142; see note at the foot of this file.* **Harvest-provenance entries**: #110–#117 (Instagram wave 1), #118–#131 (waves 2–3), #132–#134 (drain batch: Impeccable installed dependency, Hunyuan3D, Ox Alpha), **#135–#136 (Vaultwarden, Agent Zero)**, **#137–#146 (GitHub-trending wave 5: Hermes Studio, LoopX, CodeGraph, AgentMemory, QwenPaw, AstrBot, Ruflo†, 9router†, cmux†, Qwen Code)** — see harvest file + category-file notes. Full dossier coverage: waves 1–4 in `10-feature-dossiers/` (+raw-sources). †=conflict-tier.

## Categories

| File | Category | Entries |
|---|---|---|
| [01-coding-agents.md](01-coding-agents.md) | Coding agents & harnesses (terminal/IDE/cloud/companions) | 31 |
| [02-memory-context.md](02-memory-context.md) | Memory systems & context/token engineering | 10 |
| [03-search-data.md](03-search-data.md) | Search APIs, metasearch, scraping, doc extraction | 9 |
| [04-local-inference.md](04-local-inference.md) | Local inference runtimes & model tooling | 10 |
| [05-skills-plugins.md](05-skills-plugins.md) | Skill collections, plugin marketplaces, guideline packs | 28 |
| [06-orchestration.md](06-orchestration.md) | Orchestration frameworks, workflow engines, observability | 19 |
| [07-media-apps.md](07-media-apps.md) | Media gen/editing, office suites, end-user AI apps | 18 |
| [08-infra-data.md](08-infra-data.md) | Infrastructure & data platforms | 7 |
| [../09-feature-extraction.md](../09-feature-extraction.md) | Feature-extraction catalog: mechanisms worth taking from every entry (39 adopt / 58 note / 17 conflict / 32 skip) | 146 |
| [10-feature-dossiers/wave1/](10-feature-dossiers/wave1/) | Deep feature dossiers: 40 tools (33 adopt-tier + top slot-notes), per-feature verdicts against spec invariants | 40 |
| [../11-department-blueprints.md](../11-department-blueprints.md) | Feature-exploration synthesis: recurring-gap table (R1-R10 + Wave-2 R11-R16), department enrichment map, top-5 proposals | - |
| [10-feature-dossiers/wave2/](10-feature-dossiers/wave2/) | Deep dossiers: 12 conflict-tier teardowns w/ compliant re-implementation sketches + 24 note/harvest tools | 36 |
| [10-feature-dossiers/wave3/](10-feature-dossiers/wave3/) | Full-coverage confirmation dossiers: remaining skip/note entries + #132–136 drain batch (56 = 51 original + 5 drain-batch) | 56 |
| [10-feature-dossiers/wave4/](10-feature-dossiers/wave4/) | Trending-registration dossiers (#137–146): prior-art depth on existing rows and proposals, no new gap rows | 10 |
| [../99-corrections.md](../99-corrections.md) | Merge/dedupe ledger + adjudications + data-quality flags | 1 |

## How to read every entry

Columns: `#` (original dump row, stable ID) · Tool · Purpose (≤12 words) · **Dept-fit** · Status · Confidence · Official · GitHub.

- **Dept-fit**: `Dev` Development · `Res` Research · `Intel` Intelligence (model/cost) · `Info` Information/memory · `Auto` Automation · `Sec` Security · `Kernel` kernel/boot · `Product` end-user feature of LastHarness itself.
- **Status**: `ok` · `dup-of:#N` · `no-repo(closed)` (commercial/closed-source) · `no-repo(web-only)` (site/resource, no product repo) · `unverified-link` (name captured, product not yet located).
- **Confidence**: `high` (verified or certain) / `med` (training knowledge, unverified fetch) / `low` (unresolved).
- Every purpose carries an evidence basis per entry where it matters; methodology below.

## Methodology

Classification ran as a six-lane parallel verification wave (ulw-research discipline adapted to opencode `task` subagents): each lane owned ~18 consecutive dump rows, used training knowledge when confident, else one fetch/search for evidence, and never invented a purpose (unknowns would read `unclear—needs manual review`; none remained). A lead skeptic pass then adjudicated duplicates and conflicts ([../99-corrections.md](../99-corrections.md)). Run journal: `../runs/ulw-research/20260822-143607/`.

## Key findings

- **One true duplicate**: #45 oh-my-opencode ≡ #44 oh-my-openagent (repo rename) — merged.
- **SearXNG (#90) is already the adopted V1 search choice** (§16.4); Exa/Tavily/cloud-Firecrawl stay excluded-but-watched.
- **#97 deep-research-skills is already installed** in this environment's skill set — existing dependency, not a candidate.
- **#101 Odysseus AI** is the closest competitive reference to LastHarness's own product space (self-hosted AI workspace, 85.9k★, AGPL).
- **Compliance flags**: #82 pi-black (ToS-gray subscription shim); #21 community skills misnamed "Anthropic"; #84 fan-site official link.
- **Crowded-but-distinct clusters mapped**: token/context engineering (02), parallel-agent orchestration (01), Composio product family (05/07).

## Count corrections (2026-08-31)

Applied by the rescan in [`../14-research-rescan-2026-08-31.md`](../14-research-rescan-2026-08-31.md) §B. Every figure below was stale rather than wrong-at-the-time: counts were written when a wave landed and never updated as later waves appended.

| Field | Was | Now | Why |
|---|---|---|---|
| Active entries | 142 | **143** | 147 entry numbers − 2 merges (#45, #38) − 2 delists (#115, #129) = 143. The old figure double-excluded one. |
| `09` dispositions | 33 / 53 / 13 / 30 = 129 | **39 / 58 / 17 / 32 = 146** | 129 was correct at entry #130, stale since wave 5. |
| wave-3 dossiers | 51 | **56** | 51 was correct before the #132–136 drain batch appended 5. |
| wave-4 row | *absent* | **10** | The wave-4 directory existed but was never listed in this table. |

Row arithmetic, for anyone re-checking: **143 active + 3 retained-as-audit (#38, #115, #129) = 146 ledger rows**, and #45 is the only entry number with no row at all.
