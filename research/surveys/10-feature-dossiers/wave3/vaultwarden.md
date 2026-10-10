# Vaultwarden — Feature Dossier
Registry: #135 | Category: 08 | Verified: dani-garcia/vaultwarden — 66,208★, AGPL-3.0, pushed 2026-08-24 (repo-verify + API, 2026-08-25)

## Feature inventory
- Unofficial Bitwarden-compatible server written in Rust; implements the Bitwarden client API so official apps work unmodified [VERIFIED-fetched:https://api.github.com/repos/dani-garcia/vaultwarden]
- Full credential vault: passwords, identities, cards, secure notes; org collections + sharing [VERIFIED-fetched:https://github.com/dani-garcia/vaultwarden README]
- 2FA/TOTP support, emergency access, and attachment storage [REPORTED-unverified — feature list from README section titles this pass]
- Single Rust binary; SQLite default backend with MySQL/PostgreSQL options; runs on modest hardware [VERIFIED-fetched:https://github.com/dani-garcia/vaultwarden README]
- AGPL-3.0 licensed [VERIFIED-fetched:https://api.github.com/repos/dani-garcia/vaultwarden]
- Formerly bitwarden_rs; renamed at Bitwarden's trademark request [VERIFIED-fetched:https://github.com/dani-garcia/vaultwarden README]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Self-hosted credentials custody (server) | ⚙ Feature adoption | §14.5 Security Vault reference implementation (kernel/Security-dept infrastructure) | Concrete, battle-tested shape for the vault LastHarness already specs; AGPL fine for self-hosted use, restrictive for embedding |
| Bitwarden-API compatibility | 🧩 Mechanism only | Owner-surface integration option (§16.10) | Owner's existing password-manager clients could read LastHarness-custody credentials without new UI |
| Rust single-binary deployment | 🧩 Mechanism only | §16.1 packaging minimalism (cf. #130 PocketBase) | Second independent confirmation of the single-artifact deployment contract |
| Org collections / sharing | ⚙ Feature adoption | Security dept role-scoped secret sharing (§8.2 analog) | Department-scoped access to secrets mirrors allow-list scoping already specced |

## Notable absences
No built-in HA/clustering story beyond DB choice (single-instance by design). AGPL obligations apply if LastHarness ever exposed it as a service rather than self-hosting internally. Not an agent-integrated vault out of the box — no MCP/API surface for programmatic agent access; wiring one would be a §14.5-gated design task, never a worker-facing tool.
