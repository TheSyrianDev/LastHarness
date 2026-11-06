# LastEngine backends and packaging (plan, 2026-11-06)

Status: answers from the user on 2026-11-06. Not yet sent to the LastEngine session. Nothing below was checked against LastEngine's code.

## Decided by the user

1. Backend order: CUDA cleanup, CPU, Vulkan, ROCm/HIP, Metal.
2. No AMD, Apple or Intel test hardware. The user plans a LastSoft Discord to find testers and contributors (LastHarness, LastEngine, hf-dl).
3. Three packages: LastHarness alone; LastEngine alone, in its own repo with its own releases; and one bundle with both for ordinary users.

## Backends

| Backend | Covers | Notes |
|---|---|---|
| CUDA | NVIDIA | Pinned to sm_86 today. Build for 7.5, 8.0, 8.6, 8.9, 9.0 and fail clearly below the minimum. |
| CPU | everything | SIMD paths (AVX2, AVX-512, ARM NEON) chosen at run time. Also the reference the others are checked against. |
| Vulkan | AMD, Intel, NVIDIA, integrated GPUs, Windows AMD | Biggest job: compute shaders. |
| ROCm/HIP | AMD on Linux | Source close to CUDA; a translation tool can generate most of it. |
| Metal | Apple Silicon | Needs Apple hardware to verify. |

Intel is covered by Vulkan. A SYCL backend is a later option.

## Architecture

1. Backend trait: allocate, upload, matmul, attention, norms, rope, KV cache, sampling. Model code calls only the trait.
2. Move the CUDA code behind the trait with no behaviour change; existing correctness checks prove it.
3. Pick the backend at startup by probing; `--backend` and an env var override. `lastengine probe --json` lists devices, memory and usable backends (the app needs it for recommendations).
4. Split layers across CPU and GPU, since small cards need it.

## Verification for every backend

- llama.cpp teacher-forced check, at least 98% top-1 agreement.
- The lhd compatibility script (slots, prefix cache, tool-call streaming, /props, /v1/models, /health, cancel).
- Speed against llama.cpp on matched settings.

## Packaging

- `LastHarness` releases: app only; talks to any OpenAI-style endpoint (llama-swap, LastEngine, cloud).
- `LastEngine` repo and releases: the server binary per backend and OS.
- Bundle release: app plus LastEngine plus a default llama-swap or direct launch config, for ordinary users.
- Open: licence and checksum policy for the bundle; whether the app can fetch LastEngine on demand instead of bundling.

## Risks

- No AMD, Apple or Intel hardware here; those backends depend on community testers or CI runners.
- Each backend needs its own kernels, including quantised formats (tcq3) and MoE expert streaming.
- Vulkan and Metal shaders need their own compilers in the build.
