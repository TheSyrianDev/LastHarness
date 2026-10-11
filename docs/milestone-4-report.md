# Milestone 4 report: roles, departments and planning

Written by the build session while the user was away.

## What exists

New crates `lh-team` (roles, model types, departments, the engine, plugins) and `lh-mcp` (MCP client). Changes in `lh-core`, `lh-tools`, `lh-store` and `lh-tui`.

- **Roles.** A Markdown file with a short header and a short instruction line: model type or one exact model, tools, memory slice, sampling, step cap, thinking on or off. The instruction line is capped at 600 characters, because it goes with every step the role runs. Five starter roles (researcher, writer, coder, designer, planner) are written once; deleted ones stay deleted. Every save is a new version and can be rolled back.
- **Model types.** Stock frontier, mid-tier and fastest, plus your own. All can be edited or deleted. Deleting a type that roles use needs each role moved first. A type starts with no model; a run that needs an empty type stops and says so. When a model is not served, the run stops with a clear message (the default) or tries the type's fallback list and says which model it used.
- **Departments.** A Markdown file naming a lead role and members (roles or other departments). Departments nest up to four deep; cycles, unknown members and a lead that is missing are refused with the reason.
- **The engine.** The lead plans (a plan with mistakes goes back to the lead to correct, up to the attempt cap). The plan waits for your approval, or not, by the autonomy setting: plan-first, check-ins at milestones, or fire-and-forget. Steps run on the members, one at a time or up to a limit at once (1 to 16, a setting), each as soon as the steps it needs are done; steps on the same kind of model are grouped to avoid reloading. The lead checks each result against its step and the goal. A small mistake is fixed in place by a fresh instance of the same worker. A larger miss, or a failure, goes to the lead, who chooses retry, change the model, do it itself, spawn a regular subagent, or hand it to another member, or gives up. Every step has an attempt cap (default 3). An unresolved failure goes up to the caller with the whole trail of what was tried. A sub-department is a step that runs its own plan, and its failure comes back as a failed step.
- **Workers.** A worker is sent the base prompt, its own short role line and one user message: the lead's brief plus what the steps it depends on returned. Workers do not message each other. Stop, pause and resume work between model calls. Each worker uses only the tools its role lists.
- **Projects.** The assistant gets two tools: `call_department` (a job in the current folder) and `start_project` (makes the folder, using a location from the request or `~/Projects/<name>`, then runs the department there). Workers in a project are locked to the folder: an allow rule no longer waves through a command that names a path outside it. Workers never receive these two tools, so a project cannot start another project.
- **Away mode.** Nobody is asked anything: plans are approved, permission questions are answered no, every decision is logged and listed on return (`lh team away`), and a step that needed a yes is set aside while the rest carries on.
- **MCP client.** Local servers (a program, talking over standard input and output) and remote servers (HTTP, JSON or event stream), tools only. Adding a server shows the exact command or link and nothing runs until approval. Every MCP call asks. What goes to a remote server is checked for sensitive text first. A key is read from an environment variable you name, never stored. A project folder may ship `.lastharness/mcp.json`; it loads only after you trust the folder (`lh trust`), and "own" turns it off for good.
- **Plugins.** A folder of roles, departments and theme files, previewed before install; your own roles and departments stay unless you name them; every replacement is a version you can roll back. No code runs from a plugin.
- **Terminal.** `lh team ...` (roles, types, departments, settings, away, workers with pause/resume/stop, `run` to give a goal and follow it, plugins), `lh mcp ...`, `lh trust ...`.
- **API.** `/v1/team/...` for all of the above, a run's events as a stream that a late reader can replay, questions answered by posting to the run.

## System prompt size

- Base prompt: 139 tokens typical, 189 with the default tools' one-line guidelines, 234 with every tool line (limit 300, enforced by a test).
- A worker adds its role line: 58 to 84 tokens for the starters, at most about 170 for a role at the cap. A worker's whole system message is 195 to 275 tokens (a test holds it to 300).
- The lead's own prompts, measured with the same estimate: plan 295 tokens for a five-member department, closing answer up to about 580, quality check up to about 850 (it carries the start of the result), failure question 109.
- The learned block, style profile and "about you" are not capped (see parked.md); workers do not receive them.

## How it was checked

262 automated tests pass across the workspace (not counting the real embedding test), and `cargo clippy` is clean.

- Engine tests run against a scripted model with exact control: ordering, the parallel limit, grouping by model, plan correction, approval/edit/reject, small fixes versus larger misses, every remedy, the attempt cap with the failure trail, sub-departments, stop and pause, a missing model, check-ins, a parked step, and the closing answer.
- API tests run the same through the real local API, including the assistant starting a project in a named folder and the worker's tools not including the project tools.
- **Real runs** on Qwen3.6-35B-A3B-NanoPlus-fast through llama-swap (every type pointed at it), a small `pets` department, goal "explain in two plain sentences why cats purr, then give the note a five word title".
  - First run (12.5 minutes with the CPU shared): it worked end to end, with a retry and a quality check, and it showed three real flaws, all fixed: a step used another step's result without listing it as a dependency (so the worker saw nothing and invented a title); the quality check never saw the goal, so a title off the topic passed; the closing answer said a file was saved when the write had been refused.
  - Second run after the fixes (5.5 minutes): the plan listed the dependency, both steps passed the check, the title fitted, and the closing answer said no files were saved.
  - Qwen thinks a lot; the run used a 6000-token limit per call. On this machine, with another job using the CPU, it generated about 3 tokens per second.

## What is not done

See `.grill/parked.md`, section Milestone 4. The main points: the project leader is the department's lead and not a separate clone; permission questions go straight to you instead of up through each level; away mode says no instead of deciding; agents cannot yet pick or load models or propose providers (one endpoint); themes, the worker flow diagram and installing a plugin from a link are not built; MCP was tested against mock servers only; folder trust covers MCP servers and not yet project skills.
