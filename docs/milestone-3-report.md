# Milestone 3 report: learning v1

Written by the build session while the user was away.

## What exists

New crate `lh-learn`, plus learning in `lh-store`, `lh-tools`, `lh-core` and `lh-tui`.

- **Rules.** One habit in a sentence or two, with a topic, an optional project and role, a privacy class, the quotes that led to it, and where it came from (suggestion, stated by the user, typed by the user). Every version is kept in an append-only history; rollback adds a new version that copies an old one. A rule can be switched off, edited or deleted for good (its text is blanked in the history, the fact that it was deleted stays).
- **Most specific rule wins.** Rules that share a topic compete: a project and role rule beats a project rule, which beats a general one; newest wins a tie. Rules on other topics are unaffected. Sensitive rules are applied only for a model on this machine.
- **Style profile.** One short note (at most 1,500 characters) about tone and manner, with the same history and rollback. It is sent only to a cloud model if it contains nothing sensitive.
- **Signals and habits.** Each message you send is checked cheaply for a rule ("always", "never", "from now on", ...) or a correction ("no, I said...", "stop..."), in English and Arabic. Only the statement part is kept, not the question asked with it. A wish said in similar words in at least 3 different sessions (setting `--learn-threshold`) is a habit. Thumbs up and down (`/good`, `/bad <note>`) add signals; a thumbs down with words counts as a correction.
- **Suggestions.** At the end of a session (`/learn`, or the API) the model is asked, as one more turn on the conversation so a caching server reuses its cache, which of the things you said are lasting rules and which are about style. A habit seen in several sessions is put to the model with only the matching messages, nothing else. The result is a suggestion: add a rule, change a rule, add to the style profile, add a skill, or change a skill. A suggestion that repeats a waiting one adds its quotes to it and marks it a repeat. Nothing applies until you accept, edit or drop it. With silent learning on (`--learn silent`, off by default) suggestions are applied at once, marked "silent learning" in the history, and can still be rolled back.
- **Skills.** Real Agent Skills folders: a `SKILL.md` with a header (name, description, optional license, compatibility, allowed-tools, metadata) and optional `scripts/`, `references/`, `assets/`. The reader handles the YAML shapes real skills use (folded and literal block text, quoted text, lists, one nested map) and was checked against the skills that ship with Omarchy. Names, lengths and the folder-name match are checked as the format requires. The model sees only each skill's name and description at first, reads the steps with `use_skill`, and other files only with `read_skill_file`. Scripts run through the normal command tool under the normal permission rules. A long task (6 or more tool calls by default) on a local model may produce a suggestion for a new skill; a skill that was used may produce a suggested improvement. Every skill version is kept and can be rolled back.
- **The base prompt is never rewritten.** The system message is your own text exactly as given, then the learned block (rules, style, skill list), then memory. It is built once when a session starts, so the start of every prompt stays identical. What shaped each session is written to the activity log.
- **Terminal.** `lh learn ...` (rules, style, skills, suggestions, habits, signals; `lh learn help`), a `/learn` screen to accept, drop, postpone or flip sensitivity, and `/good` and `/bad`. The program starts as `lh` or `lastharness` (same program, spec section 4.3).

## How it was checked

187 automated tests pass across the workspace and `cargo clippy` is clean.

- **Real run, exit rule** (Qwen3.6-35B, the model AGENTS.md names for non-coding work): with no rules, "Give me three tips for sleeping better" came back as a list (3 list lines). After the same wish ("no bullet points") in three separate sessions, one suggestion was waiting and nothing had changed. After accepting it, the same question came back with 0 list lines. Switching it off brought the list back (3 lines). The history shows every version. The suggestion kept the strength of the user's words ("Never use bullet points or lists; use plain prose.").
- **Skill format.** The reader parses the real skills from Omarchy (folded descriptions, extra files) and writes headers that read back unchanged.
- **Found and fixed during the real runs:** the signal kept the whole message, so the question asked with a wish hid that three sessions said the same thing (the habit step never fired); the near-duplicate suggestions piled up; the embedding server crashed when a chat model filled the GPU (it now runs with the GPU hidden).

## What is not done

See `.grill/parked.md`, section Milestone 3: near-duplicate suggestions with different wording are compared by words, not meaning; rules are in the database and not mirrored as Markdown files; whole-state snapshots; switching skills off; roles; project skill folders.
