# Spike 2: RAM of a minimal Rust terminal app

Question: how much memory does a bare terminal chat app use, idle and in a long session?

What was built: `src/main.rs`, a throwaway chat TUI (ratatui 0.29, crossterm 0.28, tokio, reqwest with rustls). It streams from llama-swap on port 9292 and keeps the full history in memory. Release build, Rust 1.98.1 from the Arch package.

How it was measured: `drive.py` runs the app in a pseudo-terminal (50 x 160), types 30 prompts one after another (each waits 9 s for the reply, at most 400 tokens, model LFM2.5-2.6B-fast), and reads `Pss` from `/proc/<pid>/smaps_rollup`. PSS counts shared libraries fairly, RSS does not. Output is in `drive.out`.

| Point | PSS | RSS |
|---|---|---|
| Start | 3.3 MB | 5.6 MB |
| Idle 10 s | 3.3 MB | 5.6 MB |
| After turn 1 | 4.3 MB | 7.5 MB |
| After turn 10 | 5.0 MB | 8.2 MB |
| After turn 30 | 5.7 MB | 8.9 MB |
| Idle after the session | 5.6 MB | 8.8 MB |

Answer: a minimal Rust terminal client with TLS and streaming costs about 3 MB at start and under 6 MB after 30 turns of about 400 tokens each. Growth is roughly 80 kB per turn, which comes from the stored history. Memory did not return after the session ended, since the history stays in memory.

Limits of this result:
- This is the front end only. The real core will add the provider layer, memory index, SQLite and the embedding model. The embedding model is the part that can be large, and it is measured in spike 3.
- Only 30 turns, and replies were short because the model spends most of its tokens thinking. A longer session with long replies will grow faster.
- Idle CPU wake-ups were not measured here. The app polls every 30 ms and redraws every loop, so it is not a model of the idle mode in the spec.
- The build and dependency download needed the network (crates.io).
