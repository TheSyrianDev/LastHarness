# Vercel AI SDK — Feature Dossier
Registry: #102 | Category: 06 | Verified: 26,392 stars, license NOASSERTION per GitHub API (root README is a pointer stub; package README fetched instead), pushed 2026-08-24 (GitHub API 2026-08-24)

Skip-confirmation dossier. Recorded disposition: TypeScript runtime mismatch; provider abstraction already solved via llama-swap aliases. This dossier confirms both briefly from the primary source.

## Feature inventory
- **Provider-agnostic TypeScript toolkit**: "provider-agnostic TypeScript toolkit designed to help you build AI-powered applications and agents using popular UI frameworks like Next.js, React, Svelte, Vue, Angular, and runtimes like Node.js"; requires Node.js 22+ [VERIFIED-fetched:https://raw.githubusercontent.com/vercel/ai/main/packages/ai/README.md]
- **Unified provider API with Vercel AI Gateway as DEFAULT route**: model strings like `model: 'anthropic/claude-opus-4.6'` route through "the Vercel AI Gateway ... to give you access to all major providers out of the box"; direct provider SDK packages (`@ai-sdk/openai`, `@ai-sdk/anthropic`, `@ai-sdk/google`) are the opt-out path [VERIFIED-fetched:https://raw.githubusercontent.com/vercel/ai/main/packages/ai/README.md]
- **Agent primitive**: `ToolLoopAgent` class combining model + system prompt + typed tools; example wires a `localShell` tool to an external sandbox (Vercel Sandbox) for command execution [VERIFIED-fetched:https://raw.githubusercontent.com/vercel/ai/main/packages/ai/README.md]
- **Structured output**: `generateText` + `Output.object({schema: z.object(...)})` for Zod-schema-constrained results [VERIFIED-fetched:https://raw.githubusercontent.com/vercel/ai/main/packages/ai/README.md]
- **UI streaming integration**: framework-agnostic hooks (`@ai-sdk/react`) with a `UIToolInvocation` state machine (input-available → output-available) and `createAgentUIStreamResponse` route helper for chat UIs [VERIFIED-fetched:https://raw.githubusercontent.com/vercel/ai/main/packages/ai/README.md]
- **Coding-agent skill distribution**: `npx skills add vercel/ai` installs an AI-SDK skill into coding agents such as Claude Code or Cursor [VERIFIED-fetched:https://raw.githubusercontent.com/vercel/ai/main/packages/ai/README.md]

## Per-feature verdict
| Feature | Type | LastHarness landing | Rationale |
|---|---|---|---|
| Provider abstraction layer (unified model-string API) | ❌ Skip CONFIRMED | §16.1 llama-swap alias indirection | Recorded skip holds on both halves: (1) runtime mismatch — a TS toolkit cannot power Python/Claude-Agent-SDK ephemeral workers under §16.2's SDK lock; (2) the problem it solves is already solved at LastHarness's boundary — llama-swap maps role aliases onto local endpoints so workers carry zero provider-portability code. |
| Vercel AI Gateway default routing | ❌ Conflict-skip | — | Default egress through a hosted gateway is the anti-pattern under default-deny egress (§7.1): provider traffic flows only to local inference. It does independently validate that alias-style indirection is the industry-standard shape — LastHarness just implements it locally. |
| ToolLoopAgent / structured Output shapes | 🧩 Echo only | §16.6 Output schemas | Model+tools+system class and schema-constrained outputs are the now-conventional agent shape LastHarness's dispatch contract already expresses; no new mechanism. |
| UIToolInvocation streaming state machine | ❌ Not applicable | — | UI-framework plumbing for app builders; owner-console rendering derives from decision-log spans (§16.6 Output 3), not this stack. |
| `npx skills add` packaging | 🧩 Pattern echo | Skills-pack portability (wave-1 family) | Further confirmation that SKILL.md-style packs are the cross-agent distribution lingua franca; adds nothing beyond existing findings. |

## Notable absences
The repository root README is effectively empty (one line pointing at packages/ai/README.md), and GitHub API reports license NOASSERTION at repo level — licensing/governance claims were not verifiable from fetched primary sources and were not pursued given the recorded skip. No local/offline story exists anywhere in the package README: every default assumes hosted gateway or cloud sandboxes. Nothing here reopens the disposition; the entry stays a brief confirm-and-close.
