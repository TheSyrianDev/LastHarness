# Hardening report (partial)

Written by the build session. The pass was cut short by a usage limit. Done: the security review of the tool layer, the web door, channels, MCP trust, logs, and the sensitive-data path test. Not done: the soak result, release sizes in final form, and a second look at some items listed at the end.

## Found and fixed (each has a regression test)

- **Command classifier.** These were treated as read-only or checked wrongly, and now ask: `cat <(rm x)` and other process substitution; `env CMD`; `sed` scripts with `w`, `e` or `r`, and `sed --in-place`; `find -fprint/-ok`; `sort -o`, `tree -o`, `uniq in out`; `rg --pre`; `git diff --output`, `git config`, `git branch -D`, `git tag -d`, `git remote add`; leading `PATH=`, `LD_PRELOAD=`, `GIT_EXTERNAL_DIFF=` and similar.
- **Path check for commands.** `cat $HOME/.ssh/id_rsa`, `cat </etc/passwd`, `--file=/etc/x`, `a/../../x`, a symlink inside the project, `~user`, and brace expansion `{a,/etc/passwd}` all passed as "inside the project". Every word is now judged; what cannot be judged counts as outside.
- **Keys and the core's own data.** In assistant mode (project = home) `.ssh`, `.gnupg`, `.aws`, browser profiles and `~/.local/share/lastharness` (which holds sensitive memory notes as plain files) were readable without asking. They now always ask.
- **Environment.** `env` and `printenv` could show the core's tokens (channel tokens are read from environment variables). Commands now run with a small allow-list of variables.
- **Command output and timeouts.** Output was held fully in memory; it is now capped, and a timeout kills the whole process group, not only the shell.
- **File tools.** `write_file` used a predictable temporary name, so a planted link could redirect the write; it now uses a random, create-new name. `read_file` could hang on a named pipe; it reads ordinary files only.
- **Web fetch.** Added NAT64, 6to4, site-local, Teredo, documentation and IPv4-compatible IPv6 ranges, 192.0.0.0/24 and 198.18/15; proxies are no longer used (a proxy resolves names itself). Decimal, hex, octal and bracketed spellings of local addresses were tested and are blocked. Addresses and searches that carry a key, card or account number are refused.
- **Sensitive data on every way out.** A guard now refuses web fetch, web search, MCP tool calls and image prompts that repeat four or more words of a sensitive note or rule; channel replies are checked the same way. `sensitive_paths.rs` walks the model endpoint (memory tools, many turns, compaction, learning pass, review), web, MCP and a channel.
- **Web door.** Pairing ids were sequential and their status revealed the device id; they are random now. Logout now ends the server session. Open challenges and connections are capped. A browser can no longer change `memory.trust`, and can only open projects in real, non-hidden folders below home (assistant mode from a browser asks for every read).
- **Channels.** Errors from Telegram contained the bot token (it is in the address); addresses are stripped from all channel errors. Group chats (Telegram groups, Matrix rooms with more than two people, Signal groups) are no longer served, since a reply would reach everyone.
- **MCP folder trust.** A folder's server list is hashed when trusted; a changed list (for example after a `git pull`) is asked about again.
- **Logs.** The activity log now removes anything shaped like a key, token, bearer value, private key block or password in an address, before it is written.

## What remains

- **Soak and RAM numbers:** `crates/lh-core/tests/soak.py` was written and started (60 turns, compaction, a department run, idle wake-up count, PSS from /proc) but had not finished; run it: `python3 crates/lh-core/tests/soak.py` and add the numbers here.
- **Release build:** `cargo build --release --workspace` works (about 1 minute). Sizes before the last changes: `lhd` 24 MB, `lh` and `lastharness` 10 MB each.
- **Not yet done:** `/pair/start` can be flooded by anyone on the network (5 waiting requests); a "pairing window opened on the machine" would stop that. MCP stdio servers still inherit the core's environment. The proxy ban may bother users behind a proxy. The classifier is a list, not a shell parser; unknown programs ask, which is the safe side, but the lists should be reviewed again. The leak check is by word runs, so a paraphrase of a sensitive note is not caught. Activity-log redaction is pattern based.
- 298 tests pass; the prompt-size test is unchanged and green.

## Soak test results (added by the planning session from /tmp/claude-1000/soak/out.json)

Model LFM2.5-2.6B-fast, 60 chat turns through the real core (`lhd`), then one department run, then 60 seconds idle. Total 1069 s.

- **Memory (PSS) of lhd:** 11.5 MB at start, 14.4 MB at turn 0, 15.5 MB at turn 59 (growth of about 1 MB over 60 turns), 15.9 MB right after the department run, 15.5 MB at the end. No leak signs in this run.
- **Compaction:** 6 compactions, no errors. Largest prompt 2301 tokens.
- **Time to first token:** median 3.0 s, worst 24.3 s (the worst is likely a model load or swap; not investigated).
- **Idle:** 60 s watched, 13 threads, 1.17 voluntary context switches per second, 0 CPU seconds used. Near-zero, but not exactly zero.
- **Problems the run shows:**
  - The department run ended with status `timeout` on this 2.6B model. Not investigated; may be the model being too weak for planning or a real timeout bug.
  - 8 of 60 turns returned empty answers.
  - `cached_tokens_last` was 0, so prefix-cache hits were not confirmed in this run.

## Follow-up (second pass)

**Soak findings explained**
- **Department timeout:** not a core bug. Re-run on Qwen3.6-35B-A3B-NanoPlus-fast: status `done` in 444 s, 13 events, a real answer. The 2.6B model could not finish planning inside the 10-minute cap of the script. Memory (PSS) on that run: 11.7 MB at start, 9.1 MB after the run, 9.8 MB at the end; idle 1.2 context switches per second, 0 CPU seconds.
- **Empty answers (8 of 60):** the soak script's 400-token limit was used up while LFM2.5 was still thinking (direct calls: every probe stopped at 400 tokens with empty or near-empty content). The core reports that case as "ran out of tokens". `soak.py` now has `--max-tokens`.
- **`cached_tokens_last` = 0:** the server itself reports `cache_n: 0` for LFM2.5 even when two requests share a 440-token prefix (a recurrent hybrid model cannot reuse a prefix in llama.cpp). On Qwen3.6 the second request reports 593 cached tokens. The core passes the server's number through correctly; no test added, since it depends on the model.

**Remaining items from the first pass**
- **Pairing flood:** `/pair/start` now refuses everyone until the user runs `lh devices pair [minutes]` on the machine (default 5, at most 30). Test: `nobody_can_ask_to_pair_until_the_user_opens_a_window_on_the_machine`.
- **MCP environment:** a local server gets only PATH, HOME, USER, LANG, TERM, TZ, TMPDIR and LC_*, plus the `env` map in its config (`NAME = "env:VAR"` or a plain value). Test: `a_local_server_gets_only_the_variables_it_was_given`. `lh mcp add` has no flag for `env` yet; it is set in `mcp.json`.
- **Proxy:** still off by default; new setting `web.use_proxy` for users behind a proxy.

**Final release sizes** (`cargo build --release --workspace`, 47 s incremental): `lhd` 22.9 MB (16.8 MB stripped), `lh` and `lastharness` 9.8 MB each.

300 tests pass, clippy clean, prompt-size test green.
