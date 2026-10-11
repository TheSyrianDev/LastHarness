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
