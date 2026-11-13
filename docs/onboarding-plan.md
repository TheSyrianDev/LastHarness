# First-run onboarding plan (draft, doc only)

Status: draft. Not implemented. Omar will add detail. Flow taken from the CEO message in projects-45 (summarised; re-check the original before building).

## Flow
1. Welcome: one screen, what Last AI is, and that it runs on this computer.
2. Hardware check: read RAM, free disk and GPU presence. Say which model sizes fit (same fit rule as the Models page, `crates/lh-core/src/models_catalog.rs`).
3. Local or cloud: pick one. Local uses a model on this machine; cloud asks for a provider and a key stored in `lh-secrets`. Either can be changed later.
4. Optional extensions: list them as off by default (outside CLI workers, add-ons, schedules). Nothing is installed without a yes.
5. First chat: open a chat with the chosen model and a short prompt that shows memory and approvals.

## Notes
- Existing setup command `la setup` (twelve questions) is the terminal version of steps 2 to 4. Keep the two in step.
- No step downloads or runs anything without a yes.
- Open questions for Omar: exact wording of each screen; whether the hardware check blocks the flow or only advises.
