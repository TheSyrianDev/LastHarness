# pi-black — Feature Dossier
Registry: #82 | Category: 01 | Verified: 155★, MIT, last push 2026-08-19 (repo-verify.txt 2026-08-24)

Unofficial Pi package that makes a Claude Max/Pro subscription work inside the Pi coding agent by reproducing Claude Code's exact client request shape (protocol 2.1.224) for Anthropic OAuth traffic; a patched standalone binary remains as fallback. Recorded conflict: credential impersonation breaks §13 auth chain/auditability; ToS-gray.

## Feature inventory
- Subscription routing: routes Anthropic OAuth requests through existing Claude subscription usage "by applying Claude Code 2.1.224 request conventions" so Pi can use the user's Claude Max/Pro entitlement [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Version-pinned compatibility surfaces: Pi package valid only on Pi 0.84.1/0.84.2, standalone binary based on Pi 0.84.1, protocol pinned to CC 2.1.224; fails closed on other versions, each new version explicitly revalidated into an allowlist [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Surgical scope: replaces only the built-in Anthropic provider implementation and only transforms OAuth-token requests; preserves Pi credential storage, OAuth refresh, model behavior, tools, retries, streaming, usage accounting; API-key and non-Anthropic requests pass through unchanged [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Identity discovery: reads installation ID and account UUID from `~/.claude.json` (or `CLAUDE_CONFIG_DIR`) in memory only and adds matching request metadata; claims no copy/print/persist of those values [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Request-shape mimicry details: exact billing and Agent SDK system-block ordering, prompt-dependent `cc_version` suffix, structure-aware `cch` checksum via seeded XXH64, per-request `x-client-request-id`, Claude Code session headers [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Checksum patch containment: validates/updates only the first billing system block; user content, tool results, descriptions, nested fields cannot redirect the placeholder patch [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Standalone build pipeline: pins an immutable upstream commit, applies patches from `patches/`, compiles via upstream release builder; installer is checksum-verified with opt-out update check (`PI_BLACK_NO_UPDATE_CHECK=1`) [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Test hygiene: public CI uses fake transports only — never makes provider requests, requires no credentials [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]
- Unofficial status + ToS disclaimer: not affiliated with or endorsed by Anthropic or Pi; users supply their own credentials and determine term compliance; no tokens/identifiers included in artifacts [VERIFIED-fetched:https://raw.githubusercontent.com/paoloanzn/pi-black/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Core mechanism: subscription OAuth impersonating the Claude Code client inside another agent | ❌ Violates §13 auth chain + auditability; ToS-gray; second substrate (§16.2) | — | Forged client identity defeats attribution (the decision log could never truthfully say what made this call), and the provider's terms are worked around by design — the README's own disclaimer concedes it |
| Borrowing identity metadata (`~/.claude.json` IDs) onto foreign requests | ❌ Violates §13 auth chain | — | Attaching another product's installation/account identifiers to requests is identity fabrication even held in-memory; no compliant form |
| Fail-closed version pinning w/ explicit revalidation allowlist | 🧩 Pure architecture pattern | §14.5 vetted versioned-registration intake | Genuinely good discipline: trust nothing that silently floats; adopt as intake policy for any integration bound to an external contract version |
| In-memory-only secret handling (no copy/print/persist) | 🧩 Pure architecture pattern (containment discipline) | §14.5 vault handling rule | The *discipline* transfers to legitimate credential custody; the *use* here stays rejected |
| Structure-aware checksum containment (patch can't be redirected by content) | 🧩 Pure architecture pattern | Security-dept review heuristic | Input-independent integrity anchors are a sound pattern for any request-signing LastHarness itself performs |
| Fake-transport CI (no live credentials in tests) | 🧩 Pure architecture pattern | §16.8 gate test hygiene | Verification gates must exercise contracts without touching real credentials — directly reusable principle |
| Standalone patched-binary distribution channel | ❌ Violates SDK lock (§16.2); unsupported fork surface | — | Distributing patched forks of vendor clients multiplies unreviewable supply chain; irrelevant to LastHarness which has no foreign client to patch |

## Compliant re-implementation sketch
- Honest answer first: **don't.** There is no compliant LastHarness path to the core feature. LastHarness has no second agent whose credentials need smuggling — every worker already *is* a Claude Agent SDK subagent (§16.2) — and §13 forbids presenting any identity other than the true one. ToS exposure would remain even if the mechanics were clean.
- What transfers is the **legitimate credential-broker pattern** pi-black accidentally demonstrates well:
  - A broker holds secrets in one local vault (LastHarness: §14.5), injects them at call time, and never lets them reach logs, traces, or worker-visible context — pi-black's in-memory/no-persist rule, minus the impersonation.
  - Every externally-versioned contract LastHarness depends on gets fail-closed pinning: a changed upstream shape fails shut until explicitly revalidated — pi-black's allowlist-before-trust rule applied to connector/skill intake (§14.5).
  - Integrity checks anchor to structure, not content, so payload text can't steer a signature or patch — relevant to how the gateway authenticates its own outbound calls.
  - CI and going-live gates (§16.8) run against fake transports exclusively; no live credential ever exists in a test path.
- These four rules harden the existing gateway/vault story; none requires new mechanism, all become stated policy where the spec is currently implicit.
- Stay out: everything else — the shim itself, identity borrowing, patched binaries, and any future "make subscription X work in tool Y" request.

## Notable absences
No server-side component or audit trail — nothing records which subscription identity was exercised beyond provider-side accounting, so abuse detection is entirely the provider's job. No rate/budget guard against burning the subscription from a foreign agent. No revocation story if Anthropic changes the wire contract mid-flight beyond fail-closed breakage. Small repo (155★) with a single-maintainer surface; README is thorough but the threat model stops at "users determine compliance."
