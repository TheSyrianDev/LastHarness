# Milestone 5 report: onboarding and settings

Written by the build session while the user was away. Terminal only.

## What exists

New crate `lh-config`, plus changes in `lh-core` and `lh-tui`.

- **One list of settings.** `lh-config` holds every behaviour choice once: its key, kind of value, default, whether it applies now or at the next start, and a plain description. 25 settings in 11 groups: provider, memory, learning, team, media, web, spending, permissions, core, reports, updates. Values are checked when set and the message says what is allowed ("learning.mode must be one of: propose, silent, off"). They live in `~/.config/lastharness/config.toml`, a readable file; a setting not in the file has its default.
- **The core reads it.** `lhd` takes its starting values from the file; a command-line flag still beats the file. Spending mode and limits, the permission mode, bypass, sudo, memory, learning, search, image model and the idle and compaction settings all come from it. `lhd` starts with no flags once `lh setup` has run.
- **First run: `lh setup`.** Plain questions, one at a time, each with a safe default that Enter accepts: where the models are (it checks the address and lists what it finds), an API key if needed (stored in the system secret store, only a reference is written to the file), memory, learning, autonomy, image model, web search, spending, sudo, anonymous crash reports (with exactly what they would contain), updates. A wrong answer is explained and asked again.
- **One settings surface: `lh settings`.** Lists everything by group with the value, whether it is the default, and whether it applies now or at next start. `set key value` (or several `key=value`), `reset key`. It talks to the running core, which checks every value first (one wrong value changes nothing), writes the file, and applies the team settings at once. If the core is not running it reads and edits the file directly. Every change is in the activity log. The team settings are kept in step in both places (the file and the store the departments read).
- **Crash reports.** Off by default. When on, a crash writes a tiny record (version, system, the place in the code) to a file; `GET /v1/reports/preview` shows it and says what is never included. Nothing is sent, because there is no place to send it yet.
- **Export and import: `lh export <folder>`, `lh import <folder>`.** Memory (the Markdown files and conversations), rules, the style profile, roles and departments, as plain files in a new folder. Import shows what is in the folder and changes nothing until `--apply`. It never replaces your own roles, departments or style unless you name them; rules and notes already there are not duplicated; sensitive stays sensitive. Running it twice adds nothing.

## System prompt size

Unchanged: 139 tokens typical, 189 with the default tools, 234 with every tool line (limit 300). Settings and setup add nothing to any prompt.

## How it was checked

Tests for the settings list (every default is valid, bad values explain themselves, a broken file is reported and not replaced), the setup flow with scripted answers (defaults, re-asking, key to the secret store and not the file, unreachable address), the settings commands against a real core and with no core, and export from one core and import into another.

## What is not done

See `.grill/parked.md`, section Milestone 5. How first-run choices are presented (profiles, step by step, or just in time) is still open; `lh setup` is the step-by-step form and is easy to change. The provider cannot be switched without restarting the core. The permission mode setting is read at start. Crash reports are only prepared.
