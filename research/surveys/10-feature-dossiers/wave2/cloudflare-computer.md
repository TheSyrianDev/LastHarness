# Cloudflare Computer — Feature Dossier
Registry: #61 | Category: 08 | Verified: 8,608★, MIT, last push 2026-08-21 (repo-verify.txt 2026-08-24)

Cloudflare's agent-computer primitive: a virtual filesystem whose authoritative state lives in SQLite inside a Durable Object, projected into pluggable execution surfaces — a FUSE-mounted sandbox container, a just-bash Worker isolate shell, or a fresh ECMAScript-isolate Worker. Recorded conflict: remote third-party execution vs §16.1 isolation + §2.1 self-hosted.

## Feature inventory
- Virtual filesystem inside a Durable Object with SQLite as the authoritative store [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- Single execution entry point `workspace.runtime.exec(source, { backend })` with multiple registered backends under stable IDs, connected lazily on first use; workspace also constructable backend-less for filesystem-only use [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- Container backend: SQLite state projected into a sandbox container as a real FUSE mount by a sandbox-side daemon (`computerd`) that syncs over capnweb RPC; full Linux userland, real binaries, real network [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- Isolate-shell backend: just-bash running in a Dynamic Worker reaching the Workspace over Workers RPC — no second store or sync round trip [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- Isolate-JavaScript backend: ECMAScript module in a fresh Dynamic Worker with structured input/results, durable relative imports, configured libraries, Workspace-backed `node:fs/promises`, and trusted `ws:git` / `ws:artifacts` modules [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- Per-backend egress policies — matching `none`, `all`, or custom policies across the container, worker-shell, and worker-JavaScript backends (`examples/egress`) [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- MCP example: one Code Mode `code` tool backed by a durable workspace, a Worker shell, and a full Linux container (`examples/mcp`); chat-agent example using the workspace as working directory (`examples/think`) [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- Explicitly PREVIEW ONLY: unstable APIs, not production-suitable; docs under `docs/` are forward-looking intent, not description of current code [VERIFIED-fetched:https://raw.githubusercontent.com/cloudflare/computer/main/README.md]
- Entire surface runs on Cloudflare's runtime — Durable Objects and Dynamic Workers on Cloudflare Workers infrastructure [REPORTED-unverified as a separate claim; it is the product's defining substrate stated throughout the README]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| SQLite-authoritative VFS projected into execution surfaces | 🧩 Pure architecture pattern | §16.1 stack + §12 decision-log storage | State lives once in SQLite and is *projected* to consumers — the same discipline LastHarness already applies (one SQLite database per installation, §16.1); the projection idea extends it to sandbox filesystems |
| Single mediated `exec` entry point with pluggable backends | 🧩 Pure architecture pattern | §4 kernel dispatch boundary | One choke point through which all execution passes maps onto the kernel-owned spawn/route/teardown responsibility |
| FUSE-mount projection of SQLite state into the sandbox | 🧩 Pure architecture pattern (mechanism) | §16.1 Linux-env sandbox mounts | The *local* form of this — projecting dispatch-scoped workspace state into the WSL2/containerization/nspawn sandbox as a mounted FS — fits the existing isolation boundary without any new service |
| Container backend hosted on Cloudflare (`computerd` + capnweb sync) | ❌ Violates self-hosted/no-cloud-execution (§16.1) | — | WHERE does execution happen? In a container synced to a remote third-party host; §16.1 excludes any dispatched tool executing outside the owner's Linux environment |
| Dynamic-Worker isolate backends (shell + JavaScript) | ❌ Violates self-hosted/no-cloud-execution (§16.1) + third-party-runtime ban (§2.1) | — | Execution happens on Cloudflare Workers — precisely the "cloud worker" exclusion recorded 2026-08-11 in §16.1 |
| Per-backend egress policy (none/all/custom) | ⚙ Capability Security dept owns | §13.4.2 gateway + §7.1 egress block | Independent arrival at default-deny egress policy attached to an execution surface; strengthens the existing gateway design rather than changing it |
| MCP Code Mode example (`code` tool over durable workspace) | ⚙ Capability existing dept owns | §14.5 skills/tools registry via SDK MCP client | MCP client is already part of the unified Claude Agent SDK stack (§16.2); this is a tool-packaging shape, not new architecture |
| Durable Object residency itself | ❌ Violates standing-process ban (§16.5) + self-hosted (§2.1/§16.1) | — | A DO is resident cloud-held state serving requests continuously — both standing-infrastructure and off-premises |

## Compliant re-implementation sketch
- Extract the architecture, not the hosting: authoritative state in the single local SQLite database (§16.1), with each ephemeral worker's sandbox receiving a **dispatch-scoped projection** of exactly its allowed slice as a mounted filesystem inside the existing §16.1 Linux environment.
- Mount lifetime is bound to the dispatch: projected at worker spawn, torn down at worker termination — no `computerd`-style ever-running sync daemon (§16.5).
- Keep the single-execution-entry-point shape: every exec goes through the kernel-mediated dispatch contract (§4/§16.5), so "which backend ran what" is always answerable from the decision log.
- Adopt egress-as-backend-policy verbatim as a config idiom: `none` default, `all` forbidden, custom = named §13.4.2 gateway allow-list entries per role.
- The isolate-JS structured-input/results contract informs §16.6 Output schema validation — results that don't conform are refused, not parsed as prose.
- MCP `code`-tool packaging may inspire how compound file+exec skills register in §14.5, behind vetted versioned intake.
- Stay out: Cloudflare Durable Objects and Workers (remote third-party execution, §16.1/§2.1), the resident sync daemon, and the forward-looking spec surfaces the README itself flags as not-yet-real.

## Notable absences
No memory scoping or multi-tenant isolation story at the workspace level beyond backend IDs; no audit log of exec calls mentioned in the README (the spec docs are explicitly aspirational). No local/offline mode — the architecture presumes Cloudflare infrastructure even in examples labeled self-contained. Preview status means every feature above is design-intent-grade evidence, not shipped behavior.
