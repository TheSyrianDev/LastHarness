# Pi — Feature Dossier
Registry: #43 | Category: 01 | Verified: 96,394 stars, MIT, pushed 2026-08-24 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: loop/unified-API slots filled by Claude Agent SDK lock §16.2. The README confirms those two slots are literally Pi's first two packages — the agent runtime and the unified LLM API are the product, and both are what §16.2 already standardized on.

## Feature inventory
- **Three-package decomposition**: `pi-coding-agent` (interactive CLI), `pi-agent-core` ("Agent runtime with tool calling and state management" — the loop), `pi-ai` ("Unified multi-provider LLM API (OpenAI, Anthropic, Google, …)") [VERIFIED-fetched:https://raw.githubusercontent.com/earendil-works/pi/main/README.md]
- **No built-in permission system**: "Pi does not include a built-in permission system for restricting filesystem, process, network, or credential access. By default, it runs with the permissions of the user and process that launched it." [VERIFIED-fetched:https://raw.githubusercontent.com/earendil-works/pi/main/README.md]
- **Containerization doc with three isolation patterns**: Gondolin extension (host keeps `pi` + auth, routes tools/`!` commands into a local Linux micro-VM), plain Docker, OpenShell policy sandbox [VERIFIED-fetched:https://raw.githubusercontent.com/earendil-works/pi/main/README.md]
- **Supply-chain hardening**: exact-pinned direct deps, `min-release-age=2` to skip same-day npm releases, lockfile as ground truth, shrinkwrap with an explicit lifecycle-script allowlist that fails on unreviewed additions, scheduled `npm audit` in CI [VERIFIED-fetched:https://raw.githubusercontent.com/earendil-works/pi/main/README.md]
- **Vendor-neutral telemetry package**: `pi-telemetry` ships "telemetry contracts, reference adapter, conformance tests, and typed schemas" [VERIFIED-fetched:https://raw.githubusercontent.com/earendil-works/pi/main/README.md]
- **Session publishing to Hugging Face**: `pi-share-hf` publishes OSS coding-agent session datasets (maintainer publishes own sessions) [VERIFIED-fetched:https://raw.githubusercontent.com/earendil-works/pi/main/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Agent loop (`pi-agent-core`) + unified multi-provider API (`pi-ai`) as adoptable slots | 🧩 Mechanism only | Validates existing SDK lock §16.2 | The recorded skip confirmed feature-for-feature: these are precisely the two capabilities the Claude Agent SDK already provides LastHarness's workers; adopting Pi-the-harness would be a second substrate for zero new capability. |
| No in-harness permission system; policy delegated to launch environment | 🧩 Mechanism only | Validates §16.6 input 4 placement of enforcement at dispatch | Independent confirmation that harness-resident permission systems are the wrong layer — LastHarness scopes each worker via the dispatch-time role config slice; Pi's authors route the problem to the surrounding environment entirely. |
| Micro-VM/container sandbox patterns (Gondolin/Docker/OpenShell) | 🧩 Mechanism only | Reinforces §16.1 Linux-env boundary | Same doctrine as LastHarness's containment-is-the-environment's-job rule; local micro-VM routing matches the WSL2/systemd-nspawn/bubblewrap resolution family. |
| npm supply-chain hardening (pinning, release-age floor, script allowlist) | 🧩 Mechanism only | Skills/intake supply chain, §14.5 addendum candidate | Concrete checklist for the registry's noted gap that "the skills supply chain has no intake lifecycle in spec"; every mechanism here transfers to skill/plugin intake review. |
| Typed telemetry contracts with conformance tests | 🧩 Mechanism only | §16.6 Output 3 span schema | Validates defining the per-dispatch trace schema once and testing adapters against it — exactly the adopted LangFuse-shaped Output 3 approach. |
| HF session-dataset publishing | ❌ Conflict-skip | — | Bulk transcript egress to a public host; outside default-deny egress and irrelevant to single-owner operation. |

## Notable absences
No memory/knowledge system at all (state management is within-session). No approval/gating UX beyond the OS. No onboarding or non-technical surface. Multi-provider support is cloud-vendor-first — local llama.cpp-style endpoints exist via OpenAI-compatibility but are not a documented headline. Nothing durable crosses sessions by design, so nothing here conflicts with the immutable decision log — it simply has no counterpart.
