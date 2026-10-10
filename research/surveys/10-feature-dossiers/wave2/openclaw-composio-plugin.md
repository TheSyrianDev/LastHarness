# OpenClaw Composio Plugin — Feature Dossier (THIN-SOURCE)
Registry: #40 | Category: 05 | Verified: 44★, none, last push 2026-05-07 (repo-verify.txt 2026-08-24; README lives on `master` branch — `main` 404s)

Marketplace/plugin entry that wires OpenClaw agents to the hosted Composio platform: 1000+ SaaS tool bindings reached through the Composio CLI, with per-app account linking (`composio login` / `composio link`) and execution performed by Composio's cloud. Recorded conflict: outbound-by-default account layer vs §13.4.2/§7.1 egress default-deny.

## Feature inventory
- Access to 1000+ third-party tools via Composio — Gmail, Slack, GitHub, Notion, Linear, Jira, HubSpot, Salesforce, Google Drive, "and more" [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/openclaw-composio-plugin/master/README.md]
- Account layer: `composio login` / `composio whoami` sign-in plus per-app connection via `composio link <app>`; unconnected apps block execution until linked [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/openclaw-composio-plugin/master/README.md]
- Tool discovery + schema inspection: `composio search "create a github issue"` and `composio execute <ACTION> --get-schema` [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/openclaw-composio-plugin/master/README.md]
- Dry-run execution: `composio execute GITHUB_CREATE_AN_ISSUE --dry-run -d '{...}'` previews parameters before a live call [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/openclaw-composio-plugin/master/README.md]
- Multi-step scripted workflows: `composio run '<js>'` executes inline JavaScript calling `execute(<ACTION>)` repeatedly [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/openclaw-composio-plugin/master/README.md]
- OpenClaw plugin install (`openclaw plugins install @composio/openclaw-plugin`) providing agent/user onboarding guidance for the CLI [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/openclaw-composio-plugin/master/README.md]
- Helper commands `openclaw composio setup|status|doctor` printing install/login steps and checking CLI state [VERIFIED-fetched:https://raw.githubusercontent.com/composio-community/openclaw-composio-plugin/master/README.md]
- Hosted control plane: Composio Dashboard at app.composio.dev and docs at docs.composio.dev — the account store and tool router live on Composio's side, not locally [REPORTED-unverified — dashboard/docs links appear in the README; hosted-execution architecture is the recorded conflict basis and matches the #32 sibling dossier's finding]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| 1000+-tool catalog reached through one hosted router | ❌ Violates default-deny egress (§13.4.2) AND no-cloud-execution (§16.1) | — | WHERE does execution happen? On Composio's cloud, with a single grant spanning the whole catalog; LastHarness allows only individually owner-approved gateway crossings |
| `composio login`/`link` outbound-by-default account layer | ❌ Violates default-deny egress (§13.4.2) + credential custody (§14.5 vault) | — | Credentials for SaaS accounts are minted and held on the hosted platform, authorizing egress before any per-call allow-list decision |
| Tool search + `--get-schema` discovery | 🧩 Pure architecture pattern | §13.4.2 gateway pre-flight | A searchable action index feeding allow-list decisions is sound; in LastHarness the index describes candidate gateway connectors, it never grants access by itself |
| `--dry-run` parameter preview before live execution | ⚙ Capability Security dept owns | §13.4.2 gateway + §7.2 owner approval | Exactly the shape of a pre-flight approval gate: render the exact outbound call, require confirmation, then log the crossing |
| `composio run` multi-step JS scripts | ⚙ Capability Automation dept owns | §14.6 job_runner | Multi-step external workflows compile to Automation jobs; each external step routes through the kernel and the gateway, never one blob of unmediated script egress |
| OpenClaw plugin onboarding entry + helper commands | ⚙ Capability skills registry owns | §14.5 procedural store | Guidance content enters through vetted versioned skill registration; it instructs, it does not open network paths |
| Hosted dashboard/auth control plane | ❌ Violates no-cloud-execution (§16.1) | — | Auth brokerage and tool execution run on a third-party host; §16.1 excludes any dispatched tool whose execution happens remotely |

## Compliant re-implementation sketch
- Core value = wide SaaS reach without per-app integration work. Rebuild it as a connector catalog behind the §13.4.2 egress gateway: each target app (GitHub, Gmail, Slack…) is a discrete owner-approved `allow_list` entry in the §7.1 `egress` block — breadth of catalog, never breadth of grant.
- Credentials stay local in the §14.5 vault; nothing mints or stores SaaS tokens on a hosted platform.
- Adopt the dry-run pattern wholesale: the gateway renders the exact request (method, endpoint, payload) for §7.2 owner confirmation on first use of any new action class, then logs the crossing per §13.4.2's all-outbound-logged rule.
- Action-schema discovery (`search`/`get-schema`) becomes static, vetted metadata shipped with each connector definition — no runtime dependency on the hosted router.
- `composio run`-style multi-step flows map to Automation-department jobs (§14.6): steps are kernel-routed sub-calls, each independently allow-listed and logged.
- Onboarding helper text ships as a registered skill (§14.5) teaching the owner how to approve connectors — mirroring the plugin's guidance role without the plugin.
- Stay out: the hosted tool router entirely (execution locality, §16.1), the shared one-key-grants-all-catalog model (default-deny, §13.4.2), and any account layer that authorizes before the owner allow-lists.

## Notable absences
No self-hosting story — the CLI requires `composio login` against the hosted platform; no local execution mode documented. No egress policy, rate-limit, or audit surface in the README (THIN-SOURCE: single short README pointing at external docs; the old direct-MCP path is explicitly deprecated). No permission granularity beyond per-app linking — one link appears to expose the app's whole action surface. License field absent from the repo metadata (none).
