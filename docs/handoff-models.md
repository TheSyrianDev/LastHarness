# Handoff: models, create, web search (stopped at 98% usage)

## Commits (Last AI, no push)
- e7fc36a Machine lock: take and release through machine-lock (tested: lh-core 71 lib)
- 674b6f8 Create: image and video seeds in 0..2^31-1 (tested: lh-core 71 lib)
- 5f45b2d Docs: steal plan (proposal, not approved)
- 327307e README sections and CHANGELOG
- 84843b8 Create inline result (lh-gui 332 pass); 27c402b poster per job (lh-core 69 pass at that point)
- 858d3db web search finds SearXNG on 127.0.0.1:8888 (lh-tools 61 + 4 pass)
- 322fdc6 Wan negative prompt matches contract; seed was clamped here first

## Untested
- No live Wan or Z-Image run since the seed fix (674b6f8). The colour-noise fix is unconfirmed on this machine.
- Create GUI Generate button and inline result: not checked live. No screenshots since the GUI stop.
- README claims: written from code, not checked against a running app.
- Machine-lock path through lhd jobs: only the failure case ("busy") was seen live.

## Seed clamp
- Done in code (0..2^31-1, image and video). Live check still pending. ai-8c identified the 64-bit seed as the cause.

## Steal plan
- Written as a proposal in docs/steal-plan.md (5f45b2d). Not approved. Nothing implemented from it.

## Next step
- When `~/AI/Rig/bin/machine-lock status` shows free: run one Wan video through lhd (it takes and releases the lock itself), extract a frame, check it is not colour noise.
- Then Omar decides steal-plan item 1 (hashline edits).

## State at stop
- Lock: released by me (no line of mine held). Lastengine held it at last check.
- Models: I loaded none. Nothing started or unloaded by me in this step.
- lhd still running on 127.0.0.1:7842 (headless).
