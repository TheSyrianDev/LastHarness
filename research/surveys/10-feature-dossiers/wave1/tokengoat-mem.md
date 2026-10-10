# Token-Goat Mem — Feature Dossier
Registry: #5 | Category: 02 | Verified: ~1 star, PolyForm Noncommercial 1.0.0, last push 2026-07-14 (GitHub API 2026-08-23)

## Feature inventory
- Local-first SQLite fact store at `~/.mem/mem.db` (`TOKEN_GOAT_MEM_HOME` relocatable); zero network calls ever; short-lived CLI process, no daemon, no tray, no wizard [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Typed explicit capture: `mem remember` requires `--kind preference|decision|fact|correction`; optional paired `--subject/--value` powers contradiction detection; scope `global|project|path`; source refs recorded [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Anchor predicates re-validated on every recall: `file-exists/absent`, `file-newer-than`, `newest-of`, `file-contains/not-contains`, `glob-exists`, `git-branch-is`, `git-tracked`, `package-version` — pure read-only filesystem/git predicates yielding `affirmed | unverified | contradicted`; only affirmed is ground-truth eligible [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Trust levels + time decay: low-trust facts surfaced marked "verify"; unpinned preferences decay in confidence unless re-affirmed; `mem pin <id>` exempts from decay (still subject to contradiction/anchor suppression) [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Contradiction handling: same subject+scope with different values → recall prefers newer and hides loser; `mem epoch --gc` persists resolution; contested facts never surface as ground truth, routed to human review [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Human gating pipeline: `mem suggest` stores facts permanently `pending` (never auto-promoted); markdown import of CLAUDE.md-style bullets lands as pending/derived; promotion/rejection only via `mem review --promote/--reject` — no bulk-promote shortcut [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Secret safety: secret-pattern + entropy screening before any persist; secrets/credentials/PII/high-entropy tokens/full file contents never stored by default; layered enforcement (short extracts → screening → review) [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Audit trail + cheap polling: `mem forget` soft-deletes (marked superseded, kept for audit); monotonic write epoch bumped by every write; `mem epoch` is a read-only cache-invalidation key so hosts skip full recalls when nothing changed [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- TGMEM/2 hint wire format (`--hint-format`): self-caveating per-fact display lines + single shared footer; contested/low-trust facts excluded entirely; fail-open (no hints if mem absent or call times out); TGMEM/1 kept for compatibility [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Idempotent host wiring: `mem init/uninstall` for claude-code/codex/copilot-cli/copilot-vscode configs; re-runs upgrade mem's own entries in place, conflict-error on unstamped foreign entries; `--dry-run` supported [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]
- Full-fidelity JSON export/import (preserves id/status/confidence/timestamps, idempotent, secret-screened) vs advisory-only MD import [VERIFIED-fetched:https://raw.githubusercontent.com/DFKHelper/token-goat-mem/master/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Anchor predicates w/ freshness verdicts at recall time | ⚙ Feature adoption | Gated memory dept § | Solves the confident-stale-memory failure mode with pure sandboxed predicates (no shell-out, bounded I/O, root-confined) — directly transplantable |
| Trust levels + confidence decay + pinning | ⚙ Feature adoption | Gated memory dept § | Deny-by-default epistemics: nothing becomes ground truth until affirmed/promoted and kept fresh |
| Pending→review→promote human gate (never auto-promote) | ⚙ Feature adoption | Approval gate / memory dept § | Exact allow-list philosophy applied to memory writes |
| Contradiction detection + suppression from ground truth | 🧩 Mechanism only | Gated memory dept § | Subject/value conflict resolution pattern; winner-picking policy should be kernel-owned |
| Soft-delete-for-audit + monotonic write epoch | 🧩 Mechanism only | Decision-log § | Epoch-as-broadcast-key is a clean cache-invalidation primitive for kernel↔worker hints |
| Secret-pattern/entropy screening at capture | ⚙ Feature adoption | Egress/memory boundary § | Keeps credentials out of durable store before anything else sees them |
| Fail-open caveated hint seam (TGMEM/2) | 🧩 Mechanism only | Kernel↔worker protocol § | Self-caveating display strings are the right shape for worker-facing memory hints; fail-open must flip to fail-closed in LastHarness |
| Scope global/project/path partitioning | 🧩 Mechanism only | Department scoping § | Right idea, wrong axis — LastHarness partitions by department, not path |

## Notable absences
One flat database — no department-level partitioning or per-department access control (LastHarness's gated-memory requirement). Retrieval is relevance-ranked list only: no BM25/vector/hybrid search. Single-writer assumptions; no concurrent multi-agent write arbitration. Not yet published to npm (source install only). PolyForm Noncommercial license blocks commercial code reuse.
