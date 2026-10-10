# Corrections & Dedupe Ledger

Append-only record of entries removed or merged from the deliverable, and adjudication calls made during the skeptic pass. The original flat table is preserved verbatim at `archive/freebuff-dump-2026-08-20.md`.

## Merged / removed

| Decision | Entry | Reason |
|---|---|---|
| Merged #45 oh-my-opencode into #44 oh-my-openagent | #45 | Same repo (`code-yeongyu/oh-my-openagent`); project renamed from oh-my-opencode, npm dual-published during transition. Verified independently by lead parse (duplicate GitHub URL) and lane L3. |

## Kept-separate adjudications (not merges)

| Entries | Verdict |
|---|---|
| #32 open-claude-cowork ↔ #33 secure-openclaw | Same Composio product family (#32 bundles "Secure Clawdbot"), but distinct repos/artifacts. Both stay; cross-noted. |
| #70 LocalStudio.ai ↔ #71 KittyLitter | Companion products, same maker (runtime unifier vs phone client). Both stay. |
| #20 Open Viking ↔ #24 TencentDB-Agent-Memory | Functionally overlapping memory systems, distinct products/vendors. |
| Token/context cluster #4, #11, #12, #13 (+ skill-side #9, #10) | Distinct mechanisms: hooks interception / MCP sandboxing / reversible proxy / query-aware API / terse-output skill / YAGNI ladder. None merged. |
| Parallel-agent orchestration #76 JCode, #78 Conductor, #79 Superset, #81 Herdr | Distinct products (TUI harness / macOS app / Electron IDE / tmux-like runtime). |
| Name collisions: #79 Superset ≠ Apache Superset · #87 Buzz ≠ chidiwilliams/buzz · #83 OpenClaw ≠ pjasicek/OpenClaw | Disambiguated in category files; original table links are correct. |

## Data-quality notes

| Entry | Note |
|---|---|
| #84 Muse Code | musecodes.io is an unofficial fan site (footer disclaims affiliation); product is closed-source beta → no-repo(closed). |
| #53 Hermes Agent | 234k★ reported count looks inflated vs peer repos; possible star inflation — verify before citing adoption. |
| #82 pi-black | ToS-gray-area shim impersonating Claude Code wire protocol to spend Claude subscriptions inside Pi — security/compliance flag before any adoption. |
| #21 Anthropic Cybersecurity Skills | Community project by mukul975, NOT affiliated with Anthropic PBC despite name. |
| #104 AnyDoc | Official link resolves to parent firecrawl.dev (same domain as #74) — correct as-is. |
| #96 Competitive Ads Extractor | Skill lives INSIDE ComposioHQ/awesome-claude-skills repo — link correct, entry is a skill-within-list. |

## Harvest-wave maintenance (2026-08-22, entries #110–#117 verified)

| Entry | Note |
|---|---|
| #112 Galileo AI | **Renamed**: acquired by Google May 2025; standalone product retired 2025-06-20; successor is Google Stitch (`stitch.withgoogle.com`, free via Google Labs). Official link updated from retired galileo.ai; status stays no-repo(closed); confidence med→high. |
| #111 ViMax | **Repo path fixed**: `you-dont-know-ai/ViMax` 404s (that was the Instagram handle, not the org). Correct repo: `HKUDS/ViMax` (MIT, ~9.9k★, arXiv 2606.07649). Confidence med→high. |
| #116 Capptivo | **Status fixed**: harvest caption named the repo (`SECHAK-AG/capptivo`) but row said no-repo(closed). Verified live: MIT, ~794★, active. Status → ok with GitHub link; confidence low→high. |
| #117 Yap | **Name collision resolved**: two distinct "Yap" dictation tools exist. The harvested one (matches Instagram caption: Apple's built-in speech, runs entirely on Mac) is `FrigadeHQ/yap` — macOS 26+, Apple SpeechAnalyzer, zero network, MIT (~366★), site frigade.com/yap. The other, `AkuchiS/yap` (cross-platform Whisper-based), is a DIFFERENT tool and is NOT registered. Confidence low→high. |
| #113 DeerFlow | **Recategorized**: repo rebuilt as v2 "long-horizon SuperAgent harness" (researches/codes/creates; MIT, ~80.6k★) — no longer a deep-research/search tool. Moved from 03-search-data.md to 06-orchestration.md; purpose updated; confidence med→high. |
| #115 Handle | **Unresolved after search**: no distinct product located matching "all-in-one AI workspace" frame capture. Status corrected no-repo(closed)→unverified-link; confidence stays low; needs manual identification before any use. |
| #110 ScrapeGraphAI / #114 trycompai/crm | Verified live via GitHub API: both MIT, active (pushed within last day), 29.8k★ / 8.8k★ respectively. Confidence med→high; links unchanged. |

## Harvest wave-2 maintenance (2026-08-22, entries #118–#127)

| Entry | Note |
|---|---|
| #119 Meetily | **Repo renamed**: originally `Zackriya-Solutions/meeting-minutes`, now `Zackriya-Solutions/meetily`; older README links still point at the old name. Site meetily.ai claims ~29.7k stars. |
| #122 SpiderFoot | **Ownership note**: commercial arm acquired by Intel 471 (Mar 2026); `spiderfoot.net` domain moved to Intel471. OSS remains at `smicallef/spiderfoot` (MIT, 21.3k★) with site `spiderfoot.xyz`. |
| #127 holaOS | **License flag**: "modified Apache 2.0" with additional commercial-distribution and branding conditions — not plain Apache-2.0. Shared-memory/messaging architecture conflicts with LastHarness invariants (see 09 ledger); competitive reference only. |
| #120 FluidVoice | **License flag**: app is GPLv3 (relicensed from Apache-2.0 on 2026-02-23) but the optional local "Fluid Intelligence" enhancement runtime is proprietary/not open-source. Mirror repos (`hberends2/fluidvoice` etc.) exist with 0★ — canonical repo is `altic-dev/FluidVoice`. |
| #121 Needle 2 | Weights on Hugging Face (`Cactus-Compute/needle-2`), engine at `cactus-compute/needle`, arXiv 2607.18363. Cross-referenced from X-bookmarks harvest (`harvest-x-2026-08-22.md`). Apache-2.0. |

### Wave-2 candidates NOT registered (with reasons)

| Candidate | Reason |
|---|---|
| OpenWorker (Andrew Ng) | Duplicate — already #60; harvest post is confirmation only. |
| "HARNESS" video post | Almost certainly Deepseek Harness — already #29. Needs-video anyway. |
| Qiaomu-seo | Registered late in the same pass as **#128** (`joeseesun/qiaomu-seo`, MIT, 388★, created 2026-08-03, only 2 commits — very young repo, re-check activity before relying on it). |
| backboard.io "Recursive" | Product-demo caption, no identifiable product/repo found. Skipped as unidentifiable. |
| Moda (AI design tool) | Name too ambiguous to resolve safely; no verified repo/site. Skipped. |
| mascot generator + company mascot generator | Unnamed products in captions; nothing to verify or register. |
| 100+ production agents repo / free dev resources list / open source mac apps roundup | Roundup posts without a single named tool. |
| llms.txt commentary | llms.txt is a standard (llmstxt.org), not a tool; commentary post. |
| NEEDS-VIDEO-REVIEW batch (18 posts) | Tool names visible only in video frames per harvest file — left for manual video review. |

## Harvest wave-3 maintenance (2026-08-22, entries #129–#130)

| Entry | Note |
|---|---|
| #129 AgentReach | **Claim failed verification**: video frame said "~68k★, #1 GitHub trending, Python 3.10+" but GitHub search finds NO such repo (18 named agentreach; top = `bojieli/agentreach`, 86★, Go, SSH-agent tool; closest description match `tenlifejosh/agentreach`, 9★, Python, "persistent authenticated web access"). Registered against the closest match at `unverified-link`/`low`. Suspected engagement-bait stat inflation — same class as the #53 star-inflation flag. Re-verify manually before any use. |
| #130 PocketBase | Canonical org casing is lowercase `pocketbase/pocketbase` (task cited "PocketBase/pocketbase"; resolves identically). Verified: MIT, Go, 60,770★, pushed 2026-08-22. |
| Website Downloader | `Ahmadibrahim/Website-downloader` returns **404 (dead link)**. Closest live wget-based site cloner (`Qfas1726/Website-downloader`) is 0★/unlicensed. Recorded as flags-note in 03-search-data.md only, per plan — no registry entry. |

## Feature-dossier wave maintenance (2026-08-23)

| Entry | Note |
|---|---|
| #92 Windsurf | **Rebrand discovered during dossier pass**: docs.windsurf.com now serves Cognition's "Devin Desktop" documentation; Linux packages install `devin-desktop` with `windsurf` as transitional alias; "Legacy Windsurf Auth" path exists. Registry entry tracks the Windsurf/Devin Desktop lineage as one. Closed-source status unchanged — all mechanics remain doc-claimed only (see dossier wave1/windsurf.md). |
| Dossier quality flags | THIN-SOURCE marked on: conductor (closed Mac app), deepseek-harness (docs thin relative to scale), deep-research-skills (local install inspected instead), cursor (partial REPORTED). No invented features detected in lead spot-checks (windsurf, conductor verified line-by-line). |

## Feature-dossier wave findings (2026-08-23)

| Entry | Note |
|---|---|
| #92 Windsurf | **REBRAND discovered during dossier work**: docs.windsurf.com now serves Cognition's "Devin Desktop" documentation; Linux packages install `devin-desktop` with `windsurf` as transitional alias; "Legacy Windsurf Auth" path exists. Registry keeps #92 as Windsurf→Devin Desktop one lineage. Closed-source caveats unchanged — every local feature still authenticates to Cognition cloud and meters credits (dossier: wave1/windsurf.md). |
| THIN-SOURCE markers | Dossiers flagged thin on: conductor (closed app), cursor (repo is issue-tracker only), deep-research-skills (sparse upstream README; supplemented by locally installed copy), deepseek-harness (README lean vs scale). Extracted less rather than padded, per quality bar. |

## Wave-3 registration (2026-08-24)

| Entry | Note |
|---|---|
| #131 img2threejs | Harvest gave `hoanho/img2threejs` — 404 (handle-vs-org mixup, same class as ViMax/#111). Canonical repo verified: `img2threejs/img2threejs`, Apache-2.0, 13.2k★, pushed 2026-08-24. Registered in 07-media-apps.md at high confidence. |

## Wave-3 dossier findings — registry integrity (2026-08-24)

| Entry | Note |
|---|---|
| #70 LocalStudio | **Repo exists now**: registry said no-repo(web-only); site links `sybil-solutions/local-studio` (Apache-2.0, ~1.7k★, pushed 2026-08-22). Status should move no-repo(web-only) → ok. |
| #71 KittyLitter | **Repo exists now**: canonical slug `dnakov/litter` redirects to `0xSero/litter` (GPLv3+store-exception, ~2.5k★, pushed 2026-08-20). Status should move → ok with repo link. |
| #38 Composio Open Agents | **DISSOLVED into stale fork**: repo at HEAD is now a ~5★ fork duplicate of vercel-labs/open-agents (#37); registered Composio credential-vault content absent at HEAD. Candidate: merge #38 into #37 or re-point #38 to historical tag. |
| #115 Handle | Delist recommended: all web/GitHub/domain searches negative; handle.ai is domain-parking. Keep only if source video is re-found. |
| #129 AgentReach | Delist recommended: verification failure stands; fallback repo (tenlifejosh/agentreach) confirmed 9★/stale since 2026-03. |
| #53 Hermes Agent | Star-inflation flag RESOLVED: API confirms ~235k★ is genuine (same class as mattpocock/skills ~235k). Flag removed-by-evidence. |
| License-null pair | #8 dictionary-of-ai-coding and #98 agent-skills-flow have NO license (GitHub API null) — verbatim reuse blocked until licensing resolves. |

## Integrity sweep applied (2026-08-24, post-Wave-3)

| Decision | Entry | Reason |
|---|---|---|
| Merged #38 Composio Open Agents into #37 Vercel Open Agents | #38 | Repo at HEAD is a ~5★ stale fork duplicate of #37; registered Composio credential-vault content absent at HEAD. Credential-decoupling mechanism survives as §7.1 egress design-note (wave2/composio-open-agents.md dossier retained). |
| Removed (delisted) #115 Handle | #115 | Unidentifiable after exhaustive search (web/GitHub/domain all negative; handle.ai = domain parking). Restore if source video re-found. Dossier retained as audit trail. |
| Removed (delisted) #129 AgentReach | #129 | Claimed ~68k★ product confirmed fictitious; fallback repo tenlifejosh/agentreach is 9★/stale since 2026-03. Restore only with primary evidence. Dossier retained as audit trail. |

Data fixes applied same pass: #70 LocalStudio repo linked (sybil-solutions/local-studio); #71 KittyLitter repo linked (0xSero/litter via dnakov redirect); #53 star-inflation warning RESOLVED (~235k★ API-verified genuine); #111 ViMax star figure corrected ~9.9k → ~12.1k; license-null flags added (#8, #98).

## Drain-batch resolution (2026-08-25)

| Entry/Post | Note |
|---|---|
| #133 Hunyuan3D | Registered from vision-resolved Tencent post. Canonical family under `Tencent-Hunyuan/` (Hunyuan3D-2 flagship ~14.6k★ + 2.1 PBR + 1.0 unified + WorldClaw agentic-scenes, pushed 2026-08-13). License NOASSERTION = custom Tencent community license — review terms before weight reuse. Harvest gave no repo; org located via API search. |
| #134 Ox Alpha | The "OX setup" post (/p/DcZumomM50o/, nocodealex) RESOLVED: Ox Alpha, free stealth frontier reasoning model (`stealth/ox-alpha` OpenRouter / `x-preview-f-free` Zen), released 2026-08-20, 1M context, anonymous provider. No model repo (weights closed) → status no-repo(closed); GitHub column points at community installer `ox-alpha-installer/ox-alpha-installer`. Route-dependent retention flagged. |
| Workdoodles (/p/DcZLVTQR4S5/) | PERMANENTLY UNRESOLVED unless re-found: vision pass named it; GitHub + web searches find no such product (only unrelated Open Doodles/Iconoodle/MascotCraft). Flagged in 07 flags; not registered. |
| leadgen (/p/DcYEhX8Dwgx/, ikramrana) | PERMANENTLY UNRESOLVED: reel describes scrape+verify email automation but names no product in caption or indexed sources; no distinct tool identifiable. Closed. |

| #132 Impeccable | Appeared in 05-skills-plugins.md between passes (registered outside this foreman's runs — user-side addition). Legitimate entry (pbakaus/impeccable, installed in local harness); backfilled missing extraction-ledger row + trimmed purpose to spec length during drain-batch verification. |

| #135 / #136 (drain batch, 2026-08-25) | #135 Vaultwarden registered — harvest's "git.radar" is the POSTER handle; canonical repo dani-garcia/vaultwarden (AGPL-3.0, ~66k★). #136 Agent Zero identified from nikbuildsai reel caption itself (agent0ai/agent-zero, NOASSERTION license, ~19k★) — Docker-execution conflict noted. gittrend.io 3D-components post left flagged: gittrend is an aggregator, no distinct product named. |


## Wave-5 trending registrations (2026-08-25)

| Entry | Note |
|---|---|
| #137–#146 | Ten GitHub-trending finds registered after API verification (stars/license/pushed all fresh). License flags: #137 NOASSERTION, #142 AGPL-3.0 (self-host OK / embed restrictive), #145 license-NULL. |
| #53 stat refresh | Hermes Agent now 236,302★ (API 2026-08-25); genuine-growth trajectory confirmed. |
| #146 decision | QwenLM/qwen-code registered as distinct product (Alibaba terminal agent, Apache-2.0, 27.4k★) — not adjacent to any registered Qwen ecosystem entry (MiMo Code #51 is Xiaomi). |
| Tier-B pre-dispositions | #143 ruflo + #144 9router registered WITH conflict expectations per triage; dossiers carry full teardown treatment. |

| #137 / #145 license follow-up (dossier pass) | #137 hermes-studio NOASSERTION resolves to **BSL-1.1** (Business Source License, source-available): pattern-level prior art only — no code reuse. #145 cmux: README claims GPL-3.0-or-later but GitHub API reports NOASSERTION — flag stands until LICENSE file inspected in-repo. |

## Wave-6 registration (2026-08-30)

| Entry/Post | Note |
|---|---|
| #147 Codebuff + Freebuff | **Single-registry intake covers both products** — `CodebuffAI/codebuff` is a 301 redirect to `CodebuffAI/freebuff` (GitHub API confirms identical metadata: 11,232★ / 1,204 forks / Apache-2.0 / pushed 2026-08-30 / same HEAD commit on both URLs). Mirrors Windsurf→Devin Desktop precedent (#92). Inline dossier at [`01-coding-agents.md`](archive/01-coding-agents.md) §"#147 Codebuff + Freebuff"; adoption row at [`09-feature-extraction.md`](09-feature-extraction.md) §F (wave-6 addendum). Canonical repo: `CodebuffAI/freebuff`. The two products on top of one engine: **Codebuff** (`codebuff.com`) = open framework + SDK + 150-agent marketplace + paid agent API; **Freebuff** (`freebuff.com`) = ad-supported free tier with Desktop/CLI/Web/Cloud/Chat variants and a default-on data training clause on included model tiers. **Disambiguation — user-install vs company-operated** (correction to the initial draft, same date): at the **user-install surface**, Freebuff Cloud's remote-agent execution is a §16.1 cloud-worker exclusion; at the **company-operated surface** (BAROOD-AI / LASTHARNESS paid/ad-supported tiers), it is *not* a conflict and is the pattern-donor for company-hosted LastHarness instances. The spec is silent on the company surface; this is a flagged gap for §17.1, not a blocker. |
| Verified SDK | `@codebuff/sdk` v0.10.7 on npm (Apache-2.0, GitHub-OIDC trusted publisher, Node ≥18, published 2026-08-30). |
| Pattern adopted (no code reuse) | Typed `AgentDefinition` with zod `inputSchema`/`outputSchema`; per-agent `publisher` field; enumerated `spawnableAgents`; per-agent `compactContext` + `windowedFileReads` + `includeMessageHistory` flags; `providerOptions.data_collection: 'deny'` per agent; `.agents/` folder convention; `getCustomToolDefinition` typed tool API; `handleEvent(event)` typed event stream. All §14.5 / §16.6 / §7.1 prior art — no runtime adoption. |
| Meta-pattern addition | "Typed artifact beats typed protocol" — every wave 5–6 adoptable mechanism is a single typed object. Spec should specify shapes, not protocols. |

### Wave-6 correction (2026-08-30, same day)

The first wave-6 draft claimed Freebuff Cloud as a flat §16.1 cloud-worker exclusion, citing the §C conflict table. That overstated the conflict: §16.1 governs the **user install**, but BAROOD-AI / LASTHARNESS as a company plans paid tiers and ad-supported tiers per owner statement, which would run on company-operated infrastructure — and §17.1 is silent on whether company-hosted LastHarness instances are within the spec. The dossier and ledger row now distinguish the two surfaces; the §C conflict table is unchanged for user-install products; the company-operated surface is a separate spec-line question, flagged in the dossier for §17.1. The Codebuff *framework* and *SDK* were not affected and remain pattern-donor only.

---

## Spec extraction PR — wave-1..6 synthesis (2026-08-30)

| Change | Source DRAFT | Notes |
|---|---|---|
| 10 spec changes proposed | [`proposed-spec-extraction-2026-08-30-DRAFT.md`](proposed-spec-extraction-2026-08-30-DRAFT.md) | Single reviewable bundle. All are refinements/addenda; no locked-invariant changes (§16.1, §16.5, §16.9, §12.6). |

**Sections proposed (with origin evidence):**

- **§16.10 Owner Surfaces** — re-drafted from `proposed-section-16-10-DRAFT.md` (2026-08-24), sharpened with wave-5/6 strengthenings + new §16.10.7 worker-event stream substrate (Codebuff #147 prior art). Lands the wave-1 Owner Console & Approval Gate proposal.
- **§14.5.A Skills Intake Lifecycle** — wave-1 R5×8 (skill-pack supply chain, the single most-recurred cluster), strengthened by wave-2 oh-my-hermes + qiaomu-seo freshness, wave-3 pin/quarantine, wave-5 provenance convergence. Closes the gap named twice before in `11-department-blueprints.md`.
- **§7.1.B Compaction Protocol** — wave-1 R3×5, wave-5 LoopX + Codebuff prior art. Specifies *what* compaction produces and *how* dispatch continues; existing `context_compaction_threshold` (§7.1) names only *when*.
- **§16.6.B Result Schemas v2** — wave-1 R6×3 + R10×4 + R12. Extends the 2026-08-12 addendum (per-role success schemas) with typed failure channels and the evidence-ledger append-policy.
- **§16.6.A Kernel-composed handoff packet** — wave-2 R11 + wave-5 LoopX #138 prior art. First-class artifact type for in-dispatch delegation; closes a §4.4-vs-§16.5 ambiguity.
- **§12.2.A Evidence-strength column** — wave-2 R12. Three-tier (strong/weak/blank) classification on `decision_log` rows; pairs with §16.6.B.2.
- **§14.7.A Incident-rule DSL** — wave-2 R13 (spiderfoot prior art). Declarative match→aggregate→typed-risk rules over the event stream; narrows the rule engine to non-arbitrary computation.
- **§17.3.A Below-Low tier** — wave-2 R15 + proposal #5. Probe-generated compat matrix as Step-0 primitive; sub-4-GB devices still run LastHarness, just with a different model-resolved alias.
- **§16.1.A Single-artifact packaging contract** — wave-2 R16 (pocketbase prior art). One-file-per-platform install bundle with two-tier backup metadata (auto SQLite/Vault + manual full snapshot).

**Out of scope for this extraction (recorded for separate PR):**

- User-install vs company-operated separation for BAROOD-AI paid/ad-supported tiers. Surfaced in wave-6 dossier correction; needs a §17.1 addendum. **Blocking** the company-tier revenue model but not blocking the user-install product.
- §16.10 → §16.8 OPEN ITEM relationship (the standing-agents / persistent-inboxes / peer-to-peer exclusion): the DRAFT's §16.10.6 already restates the OPEN ITEM constraint; no new decision needed.

**Merge procedure.** All 10 sections are non-overlapping additions (no edits to existing locked sections). Merging the DRAFT into `docs/archive/lastharness-v1-spec.md` is a pure append — the spec's TOC (§0.1) would gain 10 new rows. Reviewer should validate each section's origin evidence against `research/extraction/11-department-blueprints.md` §1–§3 and `research/extraction/09-feature-extraction.md` §A–§F before merge.

### Correction to the above entry (2026-08-30, triage pass)

Two claims in the entry above are wrong, and a third is now moot. Dispositions are in [`12-spec-extraction-triage-2026-08-30.md`](12-spec-extraction-triage-2026-08-30.md).

1. **"No locked-invariant changes (§16.1, §16.5, §16.9, §12.6)" — false.** §17.3.A routes the kernel's own reasoning loop through the owner's chosen inference provider on below-Low devices. §16.1 makes kernel locality a standing property that "is not a default that yields to a global setting," and the kernel handles the §17.2 intake (DOB, religion, family/emergency contacts). Both cannot hold. **§17.3.A is rejected as drafted**; the capability probe is worth keeping, the below-Low *routing conclusion* is not. The DRAFT's closing line carries the same false claim.
2. **"Merging is a pure append, the TOC would gain 10 new rows" — false twice.** It gains **9** rows (§17.3.A is rejected), and it is **not** a pure append: six amendments require edits to existing text — §16.3 gains a scope item (it lists 8 items and does not mention §16.10 or two-tier backup), §16.6 needs a manager-as-writer clause, §16.6 Output 2 needs the `.lastHarness/` diff exclusion, and §16.10.2 needs the deny-wins vs scratch-scope contrast stated.
3. **Disposition count corrected:** 9 adopted (6 with amendments, 1 with a note, 2 clean) · 1 rejected · 0 skipped. An earlier count of "7 adopted" in the triage document itself was an arithmetic error — it did not sum to its own table, and §16.1.A had been recorded in the table with no body section. Both fixed; §16.1.A now has full reasoning and three amendments of its own.

### Merged (2026-08-30, commit `b1c7300`)

Owner sign-off given. 9 adopted sections merged into `docs/archive/lastharness-v1-spec.md` with all 6 amendments applied; §17.3.A rejected and not merged. Six locked invariant passages (§16.1 kernel locality, §16.1 isolation boundary, §16.1 INSERT-only log, §16.5 ephemeral dispatch, §12.6 immutable history, §8.2 standing exceptions) verified byte-identical against `HEAD~1`. Net +327/−14.

Owner decisions recorded in the spec: §14.7.A evaluation moved to Security's `threat_detector` (not a new `monitor` exception); owner surfaces split with Class B gates and plan artifacts deferred; Tier 1 backup in, Tier 2 a fast-follow.

**Scope reversal, recorded explicitly in §16.3.** The prior "**V1 scope: all departments** — all six ship at V1; the §16.4 precedent does not apply" decision is reversed: LastHarness is built as an MVP and scaled up. This is the second time this ledger's V1 claims have needed correction, so the reversal is written into §16.3 itself rather than only here. New structure: **M0** spine (schema, decision-log write path, one dispatch, tests) → **M1** make it a kernel (registry, routing, config loaders, memory handle, result schemas, a second department as the rework guard) → **M2+** scale-up track.

Two consequences worth carrying forward:

- **The DRAFT PR is now spent.** [`proposed-spec-extraction-2026-08-30-DRAFT.md`](proposed-spec-extraction-2026-08-30-DRAFT.md) and the triage are both historical records. Neither should be re-merged; the merged text in `docs/archive/lastharness-v1-spec.md` is authoritative and has since been amended.
- **Two systemic gaps remain unclosed and unblocked by this merge.** D1 — the "verification suite" that §11.3 and four new sections depend on is still undefined, and §16.3 item 11 (evolution sandbox) is now marked *blocked* rather than deferred on account of it. D2 — user-install vs company-operated separation for hosted BAROOD-AI tiers is still unspecced. D1 is the higher-value extraction.

### Research-count corrections (2026-08-31, commit `4fd88a0`)

Found by the full rescan in [`14-research-rescan-2026-08-31.md`](14-research-rescan-2026-08-31.md) §B. **Six stale counts across three files.** Every one is stale rather than wrong-at-the-time: counts were written when a wave landed and never revisited as later waves appended. Recorded here because `09-feature-extraction.md` and `archive/00-index.md` are historical records and should not be silently edited.

| # | File | Was | Now |
|---|---|---|---|
| 1 | `09` header dispositions | 29 adopt / 40 note / 12 conflict / 27 skip = 108 | **39 / 58 / 17 / 32 = 146** |
| 2 | `archive/00-index.md` dispositions | 33 / 53 / 13 / 30 = 129 | **39 / 58 / 17 / 32 = 146** |
| 3 | `09` appendix heading | "all 108 active entries" | **146 rows, entries #1–147** |
| 4 | wave-3 dossiers (`00-index` + `11` blueprints) | 51 | **56** (51 + #132–136 drain batch) |
| 5 | `00-index` active-entry count | 142 | **143** |
| 6 | `11` blueprints coverage claim | "COMPLETE — all 130 active entries" | **qualified: 3 entries have no dedicated dossier** |

**Two findings beyond miscounts:**

- **wave-4 was absent from `00-index.md`'s category table entirely.** The directory (`10-feature-dossiers/wave4/`, 10 dossiers) existed and was referenced nowhere in the index. Row added.
- **Three entries have no dedicated dossier**: #116 Capptivo, #118 Roboflow RF-DETR, #127 holaOS. #127 is covered indirectly via `wave4/qwenpaw.md`; #116 and #118 exist only as registry rows. None is adopt-tier, so no extraction was lost — but the "coverage is COMPLETE" claim in `11-department-blueprints.md` was not earned and is now qualified in place.

**Row arithmetic, for re-checking:** 147 entry numbers − 2 merges (#45→#44, #38→#37) − 2 delists (#115, #129) = **143 active**. 143 active + 3 retained-as-audit (#38, #115, #129) = **146 ledger rows**. #45 is the only entry number with no row.

Also corrected: `09`'s ledger footnote said *"#45/#38 merged,"* implying #38 was absent. It is present as a `note` row, retained deliberately as audit trail per the corrections history. The footnote now distinguishes the two cases.

### Tier 1 harvest into the spec (2026-08-31, commit `e5a2604`)

Four mechanisms from the rescan merged into `docs/archive/lastharness-v1-spec.md`. Net +41/−1.

- **§16.6.C Deterministic enforcement hooks** (new). Names the lifecycle-hook substrate the spec's gates depend on but never identified — adopts #2 ECC, #29 Deepseek Harness, #44 oh-my-openagent, meta-pattern §E.1. **The word "hook" appeared 0 times in the specification before this pass.** Runs in-process and dies with the session, so §16.5 is untouched.
- **§16.6 — commit-per-changeset** (adopt #91 Aider). The worker commits accepted changesets to its dispatch worktree during the session. **This closes open spec finding C2**: `success.commit_sha` now has a producer. §14.2 gains an explicit note that `merge_coordinator`'s `merge --no-ff --no-**commit**` rule governs the *integration merge*, not worktree-local commits — the two were being read as conflicted.
- **§16.6 Output 3 — dispatch trace schema** (adopt #64 LangFuse). Was one line with no schema. Now span-per-call, with `args_digest` rather than raw arguments so a trace landing in the decision log's evidence field cannot leak credentials.
- **§16.6.C — degenerate-loop breakers** (adopt #59 T-search). Repeated-identical-call, no-progress-streak, and turn-ceiling limits as a mandatory hook set. Tripping one is a `timeout`, not a `failure`, and does not spend the dispatch's retry budget.

**C2 is now closed.** Two open findings from the 2026-08-30 review are resolved by this pass (C2 directly; C1 partially, via the trace schema giving the verification suite one concrete artifact to assert against). C3–C9 remain open.

### One new observation, not yet acted on

`docs/archive/lastharness-v1-spec.md` contains three sets of **duplicate sub-headings** — `#### A. UX Narrative`, `#### B. Data Contract`, `#### C. Failure Handling` — repeated once per Appendix A onboarding step (`A` 7×, `B` and `C` 6× each; counted mechanically, see the correction below). They are scoped duplicates (each sits under a different `### Step N`), so navigation is unaffected, but they produce ambiguous markdown anchors for external deep-links. Nothing in the spec links to them, so this is cosmetic. Noted, not fixed.

### M0 built (2026-08-31, commit `3bf41cf`)

§16.3 items 1–4 are now code, not prose. **103 tests, all passing.** Eight source files under `src/lastharness/` (`schema.sql`, `db`, `sanitize`, `decision_log`, `hooks`, `worktree`, `dispatch`, `__init__`) and five test files under `tests/`.

| §16.3 item | Where | How it is proven |
|---|---|---|
| 1 — SQLite WAL schema | `schema.sql`, `db.py` | triggers + authorizer tests, reopen test |
| 2 — decision-log write path | `decision_log.py`, `sanitize.py` | gate-before-INSERT, correction semantics |
| 3 — one dispatch end to end | `dispatch.py`, `worktree.py` | 21 dispatch tests including the full path |
| 4 — tests that earn the claims | all five test files | immutability from four independent directions |

**Three decisions worth recording, because none was dictated by the spec:**

1. **Immutability is enforced twice, independently.** SQL triggers (in the database file, so they catch a raw `sqlite3` session opened by hand) *and* a Python `sqlite3` authorizer (so a dropped trigger is not silently the same as no guarantee). The suite asserts each layer separately, including a test that drops both triggers and then proves the write is *still* refused.
2. **The §12.6 Layer 1 gate was extended to the dispatch record — as redaction, not rejection.** Found while writing the dispatch tests, not from the spec: `dispatch.diff_text` stores a whole worktree diff in a table that is *not* behind the decision log's gate, and a diff is exactly the kind of text that picks up a pasted `.env`. The decision log **rejects** a secret-shaped row (its author can simply write it again); the dispatch record **redacts** it in place (the work already happened — there is nobody to ask for a cleaner version, and discarding the canonical change record would be worse than marking it). A redaction is visible in the text, naming the rule that fired.
3. **A `Finding` never carries the text it matched.** Only the rule name and the field, so the audit trail cannot itself become the leak. Tested.

**One real bug the tests caught:** `worktree.changed_files()` omitted files the worker *created*, because only `diff_from_base()` did the `git add -N` intent-to-add step. The dispatch path masked it — `Dispatcher.run()` calls `diff_from_base()` first, so the intent-to-add had already happened by the time `changed_files()` ran. An accidental ordering dependency that would have broken the moment the two calls were reordered. Fixed by extracting `_mark_untracked()` and having both functions call it.

**Deliberately absent, per §16.3:** kernel routing, department registry, YAML config loaders (one role config is inline in `dispatch.py`), the MCP memory handle, result-schema validation, the policy engine, any second department, and the real Claude Agent SDK runner — `ScriptedAgent` is the seam, so swapping in the SDK runner must not touch `dispatch.py`.

**Two known collisions carried forward, both recorded in `schema.sql` rather than only here:** the immutability triggers forbid *every* DELETE including §12.6 Layer 3's break-glass purge (M1 design work); and §12.2 names the correction field "Previous ID" while §16.1/§16.6 call it `parent_id` (open finding C9 — M0 uses `parent_id`, as §16.3 item 1 names it explicitly).

### C1 closed — the verification suite is now defined (§11.3.A)

**This was the one true blocker, and it is now closed.** §16.3 item 11 recorded the sandbox as "**Blocked, not merely deferred**" on a "verification suite" that §11.3 named and this document never defined, and it noted the gap blocked more than itself — §16.8 Step 4 routes every department's going-live validation through that same mechanism, so item 10's "gated per §16.8" had no defined gate either.

**Why it stayed undefined: the phrase was doing three different jobs.** §11.3 step 3's "run full verification suite" has three consumers asking genuinely different questions — §10.2's refinement round (*did this round fix what the new evidence said was broken?*), §14.1's kernel self-update (*does the whole system still work?*), and §16.8 Step 4's department gate (*does this department refuse its one flagged dangerous action?*). One suite cannot serve all three: running the whole regression suite to decide whether a department may refuse one action is absurd, and running one refusal assertion to decide whether a kernel update may be promoted is negligence.

**The definition:** three **layers**, and the consumer selects which run.

| Layer | Runs | Answers |
|---|---|---|
| **Invariant** | always, every change class | do the system's own properties still hold? (log immutability, allow-list denies, manager reads git, no standing process, diff excludes `.lastHarness/`) |
| **Claim** | per change | does this change survive the assertion **it** must define? (a department's flagged-invariant refusal test; a refinement's new external evidence; a release's own regression suite) |
| **Regression** | per change class | does everything already asserted still hold, against the cloned state? |

Three rules carry it: **Invariant failing makes the other layers moot**; **the suite lives in LastHarness's own repository**, not alongside it (a suite maintained apart from the code it verifies drifts from it, and a drifted suite is worse than none because it reports green); and — most important — **an empty layer fails closed**. A suite that executed zero assertions reports a failure, never a pass. The natural failure mode of a verify-before-promote gate is not a test that fails, it is a gate that finds nothing to check and waves the change through. This is §10.2's addendum applied one level up: a refinement round with no new external evidence and a suite with no assertions are the same failure.

**Deliberately *not* called "tiers."** §16.8 already defines a two-tier department gate — "Tier 1 — safety-invariant check (blocking)", "Tier 2 — task quality (non-blocking)" — and explicitly reserves that vocabulary for the department as the unit under promotion. Reusing "Tier 1" here would put two meanings behind one term in the same document. A department's Tier 1 safety check *is the Claim layer* of a §16.8 Step 4 run; §16.8 keeps the vocabulary, §11.3.A keeps the mechanism.

**Also edited:** §11.3 step 3, §10.2's addendum, §14.1's self-update sequence, and §16.8 Step 4 now point at §11.3.A; §16.3 item 11 is marked **unblocked** (the *gate* exists — the sandbox itself stays on the M2+ track). Six hunks total; **no hunk touches §16.1, §16.5, §16.6, §12.x, or §8.2**, so the locked invariant passages are provably untouched by the diff itself, not merely asserted to be.

### C10 — new finding: `§13.4.2` was cited eight times and did not exist

Found by `scripts/check_spec.py` (below), not by reading. §13.4 "Remote Access Policy" was a single three-group numbered list — Connectivity / Restrictions / Zero Trust — with no numbered sub-subsections. Eight passages cite `§13.4.2`, quoting rules that do exist verbatim as list items 2.1–2.3 ("No automatic remote access for agents", "External API calls go through a controlled gateway", "All outbound traffic is logged and rate-limited").

**Intent-true, address-false.** The rest of the document uses `§X.Y.Z` for real sub-subsections (§16.10.7, §16.6.B.1), so `§13.4.2` read as an address that was never created. It was the **only** one of 95 section numbers in the document that failed to resolve — which is why it survived three prior review passes: every other reference of that shape is real.

**Fixed in place:** the three groups promoted to `#### 13.4.1` / `13.4.2` / `13.4.3`. **No wording changed** — the sub-items flatten from `2.1/2.2/2.3` to `1/2/3` under their new heading, and all eight citations resolve. Recorded in the spec with a dated note, since silently renumbering a section is the kind of change that invalidates cross-references elsewhere.

### Owner companion devices (phone) — added §13.4.4, with one OPEN ITEM

Owner request, 2026-08-31: remote access should include a phone app to control LastHarness. **The spec had zero coverage** — every prior "phone" reference is a phone *number* in the §17.2 owner profile.

The architecture already accommodated it: §16.10.4's pairing-token ladder is device-agnostic ("a device joins by consuming a one-time pairing token … individually revocable, and the revocation ledger is itself append-only"), and §16.10.4 already routes all owner-surface traffic through §13.4. What was missing was the *name*. §13.4.4 now states that the owner's own devices — phone, tablet, laptop — are first-class remote clients, with three consequences derived from rules already in force rather than newly invented: authentication is §13.3 item 1.3 unchanged and **not relaxed per form factor** (a phone is the natural home for Class A approvals because it has the biometric hardware, but it is the same requirement); render-don't-mutate still holds (§16.10.5); sessions are logged per §13.4.1 item 3.

**OPEN ITEM — notification delivery to a disconnected phone, flagged rather than decided.** The conflict is real, not a drafting gap. §16.10.2 parks an unresolved gate and resurfaces it on reconnect; §16.10.4 excludes "third-party cloud relay of console or gate traffic"; §16.4 keeps Slack/Telegram out of V1 on the same privacy reasoning. But a phone that surfaces gates only when opened is materially less useful, and the obvious remedy — APNs/FCM push — *is* a third-party relay, and a payload carrying gate content relays exactly the traffic §16.10.4 forbids. Three options are written into §13.4.4 with their trade-offs (no push / contentless push / self-hosted delivery via UnifiedPush or ntfy). **Owner decision required.** Until one is chosen the companion client is specified for the connected case, which needs none of them.

**Not added to the §16.3 build order.** A phone client is a new build item and scope is the owner's call — recorded here for that decision rather than slotted into the locked §16.3 list unilaterally.

### Correction to an earlier entry in this ledger

The "One new observation" entry above states the duplicate Appendix A sub-headings repeat "**7× each**". Re-checked mechanically: `#### A. UX Narrative` is 7×, but `#### B. Data Contract` and `#### C. Failure Handling` are **6×** each. The claim was made from a partial count. Still cosmetic — nothing links to those anchors — but the number was wrong.

### New tool: `scripts/check_spec.py`

Dependency-free consistency checker for `docs/archive/lastharness-v1-spec.md`, run as `python scripts/check_spec.py`. Three checks: every `§X.Y[.Z]` cross-reference resolves to a heading that exists; markdown links to local files point at files that exist; no duplicate heading text at the same level. Current status: **cross-references ok (99 section numbers), local links ok, 3 duplicate headings (the cosmetic Appendix A set above).**

C10 is what this tool found on its first run, after three prior review passes had missed it. A review pass that reads for meaning does not reliably catch a reference that resolves to nothing; that is a mechanical property and it wants a mechanical check.

### §16.6 Input 3 hardcodes `llama-swap` — contradicts Path A/B provider choice (2026-08-31)

**Finding (from a competitive-capability discussion, not a spec read).** §16.6 Input 3 reads: *"the role's subagent is pointed at `llama-swap` via a model alias (e.g. `"coding"`, `"tool-calling"`), never a raw GGUF path, per §16.1."* It is written as if department/agent inference is always local. But Appendix A Step 1 (§17.2) lets the owner choose **Path A (cloud API)** or **Path B (custom OpenAI-compatible endpoint)** for department/agent inference; only *the kernel's own reasoning loop* is force-local (§16.1 standing property, line 1115). At first-run, Path A/B set `inference.base_url` to a cloud or custom endpoint — not to local `llama-swap`. So Input 3's wording is correct only under Path C.

**Same drift, two other spots.** §16.1/§17.2 already state the kernel-locality rule correctly and carve out department/agent inference as provider-following — so Input 3 is the lone holdout that assumes local. §16.6 Input 3 should read *"pointed at the configured `inference.base_url` — local `llama-swap` by default (Path C), or a Path A/B endpoint"* (keep the "never a raw GGUF path" clause, which holds in all three cases).

**Why it matters.** It touches no locked invariant (§16.1 locality is preserved — only department/agent inference was ever provider-configurable). But an M1 implementer reading Input 3 literally will hardcode the dispatch path to `llama-swap`, which would *break* Path A/B and silently contradict the first-run choice. The fix is a wording change, not a behavioral one.

**Status: OPEN — not yet applied to the spec.** Logged here for the owner/correction pass rather than edited unilaterally, consistent with the ledger's standing practice for spec-text changes. Recommends a one-line rewording of §16.6 Input 3.

### Plug-in / online-model provider surface added (§7.1.C, §17.4, Appendix A Step 1E — 2026-08-31)

Owner request, surfaced by the Hermes/OMP/Codex/Claude Code competitive review: the spec permitted cloud inference (Appendix A Step 1 Paths A/B) but had **no user-facing registration surface** for an arbitrary online model endpoint. Closed by adding the surface in **open** sections only; the §16.1 kernel-locality Locked invariant is provably untouched (registered providers are department/agent inference only, never the kernel).

- **§7.1.C Inference Provider Registry** — `providers.yaml` (list of entries: id, kind `[openai-compatible|anthropic|openrouter|custom]`, base_url, api_key_ref [Security Vault ref, never plaintext], model_aliases `[alias→provider model id]`, scope, privacy_ack, adapter, enabled). Five loader-enforced invariants: no kernel scope; privacy ack required; egress-gated; department-scoped alias resolution; key never on disk.
- **§17.4 Owner-Registered Inference Providers (Plug-ins)** — runtime surface. Non-negotiable scope boundary restated. Introduces the **two-tier UI model** the owner confirmed this session: GUI for the non-technical owner (Settings → Model Providers — read-only console render + kernel-mediated "Add provider" action, privacy-warning-gated) and a **power-user `lh` CLI** for keyboard-first operators. Both converge on `providers.yaml` and identical invariant enforcement.
- **Appendix A Step 1E Registry Persistence** — the first-run provider is written as the first `providers.yaml` entry; providers editable/removable later; additional plug-ins registerable any time without re-running onboarding.

**Code (M1 config-loader work, §16.3 item 6):** `src/lastharness/providers.py` (Provider dataclass with `__post_init__` validation; ProviderRegistry with load/save/register/remove/list/get/resolve/`kernel_providers()`), `src/lastharness/cli.py` (`lh provider add|list|remove`; `add` requires `--yes` as the privacy acknowledgement or refuses), `pyproject.toml` (`pyyaml>=6` dep + `lh` script entry). **15 new tests, all passing.** `scripts/check_spec.py` still green (101 section numbers, zero dangling refs; only the pre-existing cosmetic Appendix A duplicate-heading set remains).

**Invariant guarantees proven by tests:** a `scope: kernel` entry is rejected on construction and on load; `resolve(alias, for_kernel=True)` always returns None; `kernel_providers()` is always empty; `add` without `--yes` refuses with the Appendix A Step 1D warning.

**Aligns with the OPEN §16.6 Input 3 finding (entry above):** this registry is the mechanism that makes department/agent inference actually follow the configured provider (Path A/B or local) instead of hardcoding `llama-swap`. §16.6 Input 3 itself is left unchanged (locked §16); this registry is what an M1 implementer wires `inference.base_url` to.

Commit `da76cd9`.

## Skills keep/yeet evaluation (2026-08-31)

Owner asked to "install and use" a list of named skills. **Important grounding fact:** `Glob **/SKILL.md` → no files; `Grep` over `Research/` confirms **none of these are installable WorkBuddy skills** — they are *feature dossiers about external tools/techniques* already in `research/surveys/10-feature-dossiers/`. So "install" is not literal. The real task is: read each dossier, decide whether the **technique** is worth adopting into LastHarness, and keep or discard it. Dispositions below already exist as per-dossier verdict tables; this is the consolidated roll-up.

**Note on Caveman / "chinese mode for subagents":** Caveman's `wenyan-ultra` ("chinese") mode is **classical-Chinese compressed OUTPUT** (a token-saving output gimmick), NOT a subagent-communication accelerator. It is already dispositioned **SKIP/YEET** (net-negative tokens, standing Go process violates the one-runtime boundary, default-on telemetry violates default-deny egress). So it must NOT be installed, and "use caveman ultra chinese so subagents talk faster" is a misreading of what the tool does. Subagent-comms speed comes from the harness's own deny-by-default ephemeral-worker protocol, not from a third-party output compressor.

| # | Named item (user) | Dossier / registry | Disposition | Key reason |
|---|---|---|---|---|
| 1 | Caveman | wave3/caveman.md | **YEET / SKIP** | Net-negative tokens; standing process + default-on telemetry violate harness invariants; "chinese" is output gimmick, not comms. |
| 2 | Ponytail | wave1/ponytail.md | **PARTIAL ADOPT** | YAGNI ladder + safety carve-out = coding policy pack; subagent ruleset injection = kernel hook; `/ponytail-review` = review gate; debt ledger = task queue. |
| 3 | AwesomeDesign.md | wave1/awesome-design-md.md (#25) | **KEEP (mechanism + feature)** | Adopt `DESIGN.md` as the local design-contract artifact for the UI dept; reuse the 9-section template + AGENTS↔DESIGN split. Brand-clone library = conflict-skip. |
| 4 | Impeccable | wave3/impeccable.md (#132, Apache-2.0) | **KEEP (strong)** | 23-command design vocabulary → frontend role config; 59-rule deterministic LLM-free detector → QA gate; PRODUCT.md + DESIGN.md output → brand-context injection. Already installed in OpenCode env. |
| 5 | Taste Skill | emil-design-eng inside wave1/emil-kowalski-skills.md (#28) | **KEEP (mechanism)** | Animation build/review/improve triad + explicit do-not-animate list → UI dept motion-QA; `pick-ui-library` → dependency allow-list pattern. Swift/Sonner skills = conflict-skip. |
| 6 | Graphify | — | **NOT FOUND in research** | No dossier or mention anywhere in `Research/`. Likely a conflation; nothing to adopt. Owner to re-supply the source if it matters. |
| 7 | Headroom | wave3/headroom-cli.md | **MECHANISM-ONLY / design-note** | Reversible CCR compression, restore mechanics, KV-cache hygiene, output shaper → validates §7.1 output budget. Standing local proxy = conflict-skip. |
| 8 | TokenGoat | wave1/tokengoat.md (#4) | **KEEP (strong)** | Re-read denial + surgical-read family → retrieval/context kernel; bash-output compression → worker supervision; untrusted-content fence → egress gateway. Conflict-skip: detached indexer daemon (standing process) + PolyForm Noncommercial license (adopt patterns, not code). |
| 9 | TokenGoat Mem | wave1/tokengoat-mem.md (#5) | **KEEP (strong)** | Anchor predicates w/ freshness verdicts, trust/decay + pinning, pending→review→promote gate, contradiction suppression, secret screening → gated-memory dept. Conflict-skip: PolyForm Noncommercial. |
| 10 | GStack | wave3/gstack.md | **PARTIAL ADOPT** | 23-specialist role pack = sprint pipeline; per-repo trust triad = dept-scoped gated memory; hash-chained egress receipts = §7.1 gateway; deny-default CDP allowlist = kernel allow-list. Conflict-skip: GBrain cloud memory, `/pair-agent` ngrok, `/codex` OpenAI. |
| 11 | Everything Claude Code (ECC) | wave1/ecc.md (#2) | **KEEP (mostly mechanism)** | Role-scoped subagents w/ tool lists + model pinning → worker catalog schema; session-lifecycle memory hooks → gated memory; verification loop + checkpoint → verification gate; MCP enablement budget (<10 active) → tool allow-list sizing. No sandbox/egress enforcement in source = adopt schema, not code. |

**Additional design-skill cluster (user said "there were more design skills"):**

| Item | Dossier | Disposition | Reason |
|---|---|---|---|
| Official DESIGN.md index | wave3/official-design-md.md (#26) | **KEEP (mechanism)** | Provenance-tiered sourcing criterion (first-party > extracted > community) → adopt into §5.4; pointer-based registration w/ change-reporting → skill/design-source registry. |
| Emil Kowalski full pack | wave1/emil-kowalski-skills.md (#28) | **KEEP (mechanism)** | `review-animations` deterministic critic generalizes to any dept's output QA; `prototype` multi-version switcher → approval-gated UI; `apple-design` web-translated motion doctrine. |

**Adopted-technique → LastHarness landing summary (for M1+):**
- **DESIGN.md as single local design-contract artifact** (AwesomeDesign + Official + Impeccable) → UI dept brand-context injection.
- **Deterministic output QA gates** (Impeccable 59-rule detector, Emil `review-animations`, Ponytail `/ponytail-review`) → department output-review passes, LLM-free where possible.
- **Context/token economy** (TokenGoat surgical-read + re-read denial + bash compression, Headroom CCR, TokenGoat Mem anchor predicates) → context kernel + gated memory.
- **Subagent/worker schema** (ECC role-scoped workers, GStack role pack, Ponytail ruleset injection) → ephemeral SDK worker catalog + deny-by-default tool surface.
- **Egress/ingress hygiene** (TokenGoat untrusted-content fence, GStack egress receipts + injection classifier) → §7.1 gateway.

**Actions taken this session:** dossiers 3,4,5,8,9,11 + the design cluster read and dispositioned above. No code written yet — these are adoption decisions feeding M1/M2 design, not installable plugins. Caveman = YEET; Graphify = cannot locate.

## TUI added to `lh` (§17.4 terminal tier — 2026-08-31)

Owner: bare `lh` should open the terminal app, not error; TUI visual language inspired by Oh My Pi (omp) and Prime Agent (panel dashboard + status strip + nav menu). Built:

- **`src/lastharness/tui.py`** (new, ~280 lines): `run_tui()` keyboard-driven loop; screen builders `build_dashboard` (Panel: identity, live status strip — kernel-locality LOCKED, provider count, self-hosted mode — and the menu), `build_providers_table`, `build_departments`, `build_invariants`, `build_settings`, `build_decision_log` (reads `decision_log.db` if present). Providers screen is **functional** via `ProviderRegistry`/`Provider` — same §7.1.C validation + mandatory privacy warning (Appendix A Step 1D) as the CLI. Non-tty guard: prints dashboard and returns 0 (CI/piped shells don't hang). Placeholder palette dict pending owner branding (DESIGN.md, §5.4).
- **`src/lastharness/cli.py`** (edited): `provider` subcommand made optional; bare `lh` or `--tui` → `run_tui(args.config)`; `lh provider …` CLI unchanged and still scriptable. Top-level `--config` added for the TUI.
- **`pyproject.toml`**: dependency `rich>=13` added (single pure-Python TUI dep).
- **`docs/archive/lastharness-v1-spec.md` §17.4**: new "#### Terminal UI (`lh` TUI)" subsection documenting the terminal tier, OMP/Prime Agent inspiration, and that it's functional (not decorative). Locked §16.1 invariant untouched (TUI can only manage department/agent providers).
- **`tests/test_tui.py`** (new, 8 tests): import, dashboard/providers-table renderables, non-tty returns 0, bare-`lh`-via-`main` enters TUI, CLI path still works, decision-log reader (sqlite + missing-db). **Full light suite: 93 passed** (excluding the two worktree-heavy tests that trip the sandbox bulk-delete teardown guard — pre-existing, unrelated). Provider tests still 15/15. Smoke-tested installed `lh` (non-tty) renders the dashboard and exits 0.

`rich` already installed in the dev venv; a fresh `pip install -e .` now also pulls it.

### M1 built — kernel + first two departments (2026-08-31, commit `a6b747b`)

§16.3 items 5–9 are now code, extending the M0 spine into a kernel. **34 M1 tests pass; 160 tests pass total.** Source added under `src/lastharness/`: `schemas.py` (§16.6.B result/evidence schemas), `config.py` (§5.1/§7.1/§16.9 loaders), `registry.py` (§4.1/§16.8 lifecycle + going-live gate), `routing.py` (§4.4 kernel-mediated routing), `memory.py` (§14.5 gate + MCP handle), plus `schema.sql` gains `department_registry` + `memory_store`; `dispatch.py` wired via `RoleConfig.from_spec` + `Dispatcher.run_configured`. Departments shipped: **Development** (config-only reference impl) and **Information** (§14.5 second department, carrying `flagged_invariants: [owner_profile_no_write_without_owner]` + matching `tier1_assertions`).

| §16.3 item | Module | How it is proven |
|---|---|---|
| 5 — registry + routing + policy (logged §4.4) | `registry.py`, `routing.py` | discover/upsert preserves status; `promote` one rung; `route_task` logs every decision; worker-initiated + unknown dept/role → `refused` |
| 6 — config loaders | `config.py` | loads all three YAML shapes; unit parsers accept `min`/`sec`; fails closed |
| 7 — department-scoped memory | `memory.py` | Owner-Profile narrow ACL (`name`/`preferred_name` only); `MCPMemoryHandle` over `memory_store`; §12.6 Layer 1 now runs inside Information |
| 8 — result schemas | `schemas.py` | typed failure channels (missing `last_step` on a `failed` return = schema violation); evidence tiers strong/weak/blank |
| 9 — Information as second department | `departments/information/*` | walks `draft→active` through the gate; `flagged_invariant` demands a `tier1_assertion` |

**Two spec-faithfulness fixes (resolved, not papered over):**
1. **`failure` vs `failed` spelling.** §16.6.B.1 names the general-failure sub-channel inside `failure_schema:` as `failed:` (e.g. `failure: { refused: …; timeout: …; failed: … }`), while the dispatch terminal-status vocabulary (and the `schema.sql` decision-log CHECK) uses `failure`. The two name the same status; `TerminalStatus.coerce` now accepts both. (The alias map is module-level, not a `_ALIASES` class attribute — Python 3.13's `Enum` promotes a single-underscore class attribute into a *member*, which broke the loader.)
2. **Registry `_next_status` returned an integer order** and handed it to the enum constructor (which expects a value string), so every promotion raised `ValueError: N is not a valid LifecycleStatus`. Fixed to map the order back to its member.

**Decisions worth recording:**
- The §12.6 Layer 1 sanitization gate, which M0 ran as a direct kernel call, now executes *inside the Information department* (`MemoryGate`/`MCPMemoryHandle` call `sanitize.enforce`) — closing the §12.6 "runs inside Information" migration endpoint named in M0.
- §16.1 kernel-locality is asserted local: `ProviderRegistry.resolve(alias, for_kernel=True)` is unconditionally `None`; only department/agent inference follows the registered provider. No locked invariant in §16.1/§16.5/§16.6/§12.x was touched.
- `Dispatcher.run_configured` is the sole seam: it routes through `Router`, then short-circuits to a `refused` `DispatchResult` on a blocked route — so a refused route is a first-class outcome, not a thrown exception.

**Two known collisions carried forward (recorded in `schema.sql` + here):** the immutability triggers forbid *every* DELETE, including §12.6 Layer 3's break-glass purge (deferred to M2 design); and the §12.2 "Previous ID" / §16.1 `parent_id` naming mismatch (open finding C9) — M1 uses `parent_id` per §16.3 item 1.

### M2 built — all six departments registered and gated (2026-08-31)

§16.3 item 10 is now code: **Research (§14.3), Intelligence (§14.4), Automation (§14.6), Security (§14.7)** — each a `department.config.yaml` (§5.1) + `manager.config.yaml` (§16.9) pair, each gated per §16.8 before `active`. **22 M2 tests pass; 182 tests pass total.**

**The rework guard held.** Standing up all four required **no new kernel machinery** — `registry.py`, `config.py`, `routing.py`, `memory.py`, `schemas.py` are all unchanged M1 code. Only `KNOWN_CATEGORIES` gained the seven new memory-scope names the four departments declare. This is the M1 hypothesis being paid off: a department is a config file plus a role table.

| Department | Roles | Tier 1 flagged invariant | §16.8 gate |
|---|---|---|---|
| Research | `web_researcher`, `data_analyst`, `literature_reviewer`, `experiment_designer` | **none** — §14.3: going-live covers Tier 2 only | passes structurally |
| Intelligence | `cost_analyzer`, `quality_monitor`, `cache_manager` | `recommendation_only_no_cross_dept_writes` | passes (paired) |
| Automation | `scheduler`, `job_runner`, `monitor`, `alert_dispatcher` | `job_runner_never_dispatches_peer_workers` | passes (paired) |
| Security | `access_control_reviewer`, `threat_detector`, `audit_reviewer`, `recovery_manager` | `recovery_manager_stages_does_not_execute` | passes (paired) |

**Security's activation closes the §4.3/§5.4 non-deselectable requirement.** Kernel + Information + Security is now a satisfiable install posture, so `assert_active_required()` passes and the kernel stops refusing org-level task intake. **Registration is not activation**: a discovered config enters at `draft`, so the check stays red until each required department is walked — activating Security alone leaves it failing on Information.

**Two fidelity calls (recorded rather than papered over):**
1. **`cross_dept_reads` names departments, not fields.** The loader maps this field to the registry's `requires`, and the gate treats every entry as a registered-*department* dependency. An initial draft used §14.4's field names (`resource_accounting`, `transcript_trace`) for Intelligence and §14.7's (`development_audit_logs`, …) for Security — both would have failed the gate *and* been wrong. Corrected: Intelligence's §14.4 exceptions are **field-scoped** (resource-accounting; transcript/trace sampled), so the field is empty and the exceptions are documented in a comment rather than overstated as whole-department reads; Security's §14.7 read **is** department-wide, so it names its five peers explicitly.
2. **The per-department `policies:` label is not resolved.** `research-policy-v1`, `intelligence-policy-v1`, `automation-policy-v1`, `security-policy-v1` follow the convention M1 set with `dev-policy-v1`/`info-policy-v1`. The loader stores the string; `config/policies.yaml` remains the single enforced policy set. Per-department policy packs are **not wired, and not claimed to be.**

**Two test bugs caught by running the suite (mine, not the spec's):** asserting `information == "active"` without walking it (revealed the stronger behavior — every required department must be active — so the test now proves that); and comparing `cross_dept_reads == []` when the loader returns a `tuple`.

**Carried forward from M1 (unchanged, still open):** the immutability triggers forbid *every* DELETE including §12.6 Layer 3's break-glass purge (now deferred to M3 design — M2's scope was item 10); and the §12.2 "Previous ID" / §16.1 `parent_id` naming mismatch remains open finding C9. Also still open: §14.7.A incident-rule DSL and §14.5.A skills intake lifecycle (§16.3 item 17, neither on the critical path).

**Two real bugs found by `scripts/demo_dispatch.py` (2026-08-31, pre-commit):**

1. **`write_file` recorded `failure` after the file had already landed.** `FileTools._write` validated containment against `worktree.resolve()` (the canonical root) but built its return string with `path.relative_to(worktree)` (the *unresolved* parameter). On Windows the tempdir arrives from `%TMP%` as an 8.3 short name (`OMARBA~1`), so `Path.resolve()` re-expands it to the long form, the two spellings differ, and `relative_to` raises `ValueError` — *after* the write succeeded. The `ToolInvoker` caught it as `DispatchTerminated("failure", ...)`, so every dispatch on this host ended `failure` with `commit_sha: None` despite the file being on disk. The test suite never caught it because pytest's `tmp_path` is already long-form resolved. **Fixed two ways**: `worktree.create()` now returns `target.resolve()` (canonical at the source), and `FileTools` compares everything against a single `_root(worktree)` helper. Regression tests in `tests/test_dispatch.py` and `tests/test_worktree.py` reproduce the mismatch with a `..` segment — platform-independent, no 8.3 names needed.

2. **`list_files` was broken for every caller.** `_list` filtered with `worktree.EXCLUDED[0]`, but `worktree` is the method *parameter* (a `Path`), shadowing the imported module of the same name — so it resolved to `WindowsPath.EXCLUDED` and raised `AttributeError` on every call. It survived M0/M1/M2 because no test exercised `list_files`. The new `list_files` regression test caught it on its first run. **Fixed** by importing `EXCLUDED` by name (`from .worktree import EXCLUDED`) and filtering against the full set, not just `[0]`. Added `test_list_files_hides_dispatch_internal_state` (§7.1.B) and `test_read_file_refuses_a_missing_path` to close the FileTools coverage gap.

**First real AgentRunner built — v0.3.0 (2026-08-31):**

`runner.OpenAICompatibleRunner` + `lh chat` + `scripts/mock_llm_server.py`. The
M0 seam held exactly as designed: `Dispatcher`, hooks, worktree, memory gate,
and decision log are all untouched — the runner is a new implementation of the
existing `AgentRunner` Protocol, nothing else. Three judgment calls worth
recording:

1. **JSON-in-content tool protocol, not native function-calling.** Any
   instruct model behind llama-server can emit one JSON object; native
   tool-calling would exclude most local models. The runner corrects
   unparseable replies and off-allow-list tool requests *in the loop* (the
   model sees a legible refusal and can route around it) — the §16.6.C hook
   chain remains the hard backstop, so the soft path can't weaken it.
2. **`api_key_ref` refuses what it can't resolve.** §7.1.C says reference,
   never plaintext. Only `env:VAR` resolves today; a `vault:` reference raises
   rather than sending an unauthenticated request. The Security Vault resolver
   is a later milestone and this is now a loud gap, not a silent one.
3. **`lh chat` is one ephemeral dispatch per message (§16.5).** No standing
   agent, no session state in the runner — continuity is department-scoped
   memory only. The REPL loop is sugar over repeated `run_configured` calls.

**Two test bugs caught by running the suite (mine, not the spec's):** the
FakeModel stored the payload by reference, so assertions on "what the second
request contained" saw the *final* mutated conversation; fixed with a JSON
snapshot. And the §16.1 structural guard grepped `runner.py`'s source for
`for_kernel` — which its own docstring discusses in prose; fixed to an AST
check (no `providers`/`routing` import, no `resolve()` call).

*End of corrections ledger.*


---

## M3 — Evolution Sandbox built (2026-09-01)

Closes §16.3 item 11, the unblocked-but-unbuilt milestone. §11.3.A (the three-layer
verification suite) had already defined the gate; this builds the sandbox that consumes it.

- **Code:** `src/lastharness/evolution.py` — `EvolutionSandbox` (propose / sandbox /
  promote / discard) + `VerificationSuite` (Invariant / Claim / Regression layers).
  `sandbox()` clones current system state into a fresh git worktree (`worktree.create`),
  applies a `ProposedChange` (file writes), runs the suite, records an
  `evolution-verification` row with per-layer counts, tears the worktree down. Every run
  — pass, fail, or empty — lands immutably in `decision_log` (§11.3 step 6).
- **Fail-closed on empty (§11.3.A):** a *selected* layer that executes zero assertions is a
  `fail`, never a pass — proven by `tests/test_evolution.py::test_empty_selected_layer_fails_closed`.
  The empty early-stage suite is *not* selected (Invariant always; Claim only when the
  proposal names one; Regression only once checks exist), so it never trips the absurd case.
- **Seeded invariants (§11.3.A "Seeded at M0 by the assertions §16.3 item 4 requires"):**
  `artifact_diff_excludes_lastHarness` (§7.1.B, probed live) and `decision_log_immutable`
  (§12.1, probed via `db.raw_connection` so the trigger, not the authorizer, is exercised).
- **Tests:** `tests/test_evolution.py` — 11 tests, all passing. `scripts/demo_evolution.py`
  proves the loop offline (propose→pass→promote and propose→fail→discard) with no model/network.
- **Surface:** `lh evolution` renders the immutable evolution history from the state DB.
- **Tests count:** 211 total (200 prior + 11 M3). Spec §16.3 item 11 gained a build-status blockquote.
- **Locked invariants untouched:** the sandbox clones state read-only for verification and never
  mutates the live repo; §16.1/§16.5/§16.6 unchanged.

**Carried-forward / open after M3:**
- §16.8 Step 4's going-live gate already runs at M2 via `validate_going_living`'s injected
  `tier1_runner` seam; M3's `VerificationSuite` is the natural body for that seam — wiring it
  (the §16.8 item-10 consumer) and the §14.1 kernel-self-update consumer are the next callers,
  not new kernel machinery.
- **Test-environment note (pre-existing, not a regression):** the full *combined* pytest run
  trips the sandbox's `SAFE_DELETE_BULK_CONFIRM_REQUIRED` guard during `test_dispatch` /
  `test_immutability` teardown, when accumulated git-worktree files cross the bulk-delete
  threshold (~1,600 ops). These two are already excluded from the project's "light suite"
  (per 2026-08-31 notes). Per-file and the light suite (those two excluded) are green. M3's
  own tests remove only single worktrees and do not trip it.
