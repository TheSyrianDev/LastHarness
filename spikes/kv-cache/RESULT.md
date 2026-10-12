# Spike: saving the KV cache per chat

Question: can a local chat be continued fast by saving the model's KV cache (the prefill) and loading it later, through llama-swap, and what does it cost?

## What was tested

Model: Qwen3.6-35B-A3B NanoPlus, the router entry `Qwen3.6-35B-A3B-NanoPlus-balanced` (context 65536, KV q8_0, one slot, experts partly on the CPU), on the RTX 3050 4 GB laptop. The engine is the gs-mce build behind llama-swap on port 9292. `kv_measure.py` builds a text of about N tokens, reads it (cold), saves the slot to a file, wipes the slot, restores the file, and sends the chat's next turn. `swap_test.py` checks what a model swap does to saved slots. Everything stayed on this machine.

## Can the core reach it?

Yes. llama-swap passes `GET` and `POST /upstream/<model>/slots...` straight to the running server:

- `GET  /upstream/<model>/slots` shows the slot (cached token count, context size).
- `POST /upstream/<model>/slots/0?action=save` with `{"filename": "x.bin"}`, then `action=restore` and `action=erase`.
- The file is written into the server's `--slot-save-path`. In this router that is the folder slotkeeper picked, `~/AI/cache/slots/<hash>/`, which is named after the model command line. `GET /upstream/<model>/slotkeeper` reports it.
- The model has to be loaded first: any request loads it (17 s here, the file was in the page cache).

The router already runs a wrapper called slotkeeper in front of the Qwen entries. It saves and restores a "session" slot and prefix snapshots by itself, but only for the latest conversation, and it re-reads prompts itself on normal chat requests. So a clean measurement uses the plain `/completion` path, which slotkeeper forwards untouched. A product feature would need its own file names and its own bookkeeping per chat, and must not fight slotkeeper.

## Numbers (restore time against reading the prompt again)

| Chat size | Read again (cold) | Cache file | Save | Restore | First reply after restore |
|---|---|---|---|---|---|
| 4k tokens (3,947) | 41 s | 104 MiB | 0.05 s | 0.04 s | 0.7 s |
| 16k tokens (16,264) | 176 s | 232 MiB | 0.11 s | 0.09 s | 0.7 s |
| 32k tokens (32,778) | 375 s | 404 MiB | 0.24 s | 0.19 s | 0.7 s |

- Reading from nothing runs at about 90 to 100 tokens a second on this machine, a little slower for longer chats. A 32k chat costs over six minutes every time it is loaded again.
- With the file restored, the next turn started after 0.7 s with every earlier token reused (`cache_n` equal to the whole chat). The next turn without it took as long as the first read.
- The file size is about 63 MiB fixed (the recurrent state of this model type) plus about 11 KB per token. A 64k chat would be about 750 MiB. These are for this model, KV q8_0. Other models differ a lot.
- Restoring takes well under a second even at 32k, so the cost of the feature is disk space, not time.

## A model swap

- Swapping to another model unloads the server, so the slot in memory is gone (after swapping back the slot held only the new short prompt).
- The saved file stays on disk, and restoring it after the swap and reload worked at once (1,588 tokens, 0.06 s).
- So saved caches survive swaps, restarts and reboots, as long as the model file and its settings are unchanged. The folder name comes from the model command line, so a changed setting means a different folder and the old file is never loaded by mistake. A cache is never usable for a different model, which matches what the user decided: switching a chat to another model still uses the warning, and the whole chat is read again.

## What it means for the design (a recommendation, not a decision)

- Worth building for local models: 6 minutes down to under 1 second for a long chat is the biggest everyday saving found so far.
- Save after a turn finishes, to a name made from the chat id and the model command-line hash. Keep one file per chat (replace on each save).
- The cap matters: 400 MiB for a 32k chat on this model, so ten long chats are 4 GB. The proposed 5 GB cap with oldest-first removal fits.
- Privacy as decided: file mode 0600, delete with the chat, skip incognito, never for cloud models. The folder under `~/AI/cache` belongs to the router, not to LastHarness; the core would need its own folder passed with `--slot-save-path`, or the router's folder with names the core owns. The core cannot read the model command line, so it needs the model's slot-save folder from the router or from its own setting.
- Open for the build: how the core finds the slot-save folder, and how it behaves with a router that has no slots (cloud, LM Studio, Ollama): the feature must simply stay off.

## In the product (2026-10-11, after the build)

The core now saves and restores a local chat's cache by itself (`crates/lh-core/src/kv_cache.rs`, settings `kvcache.enabled`, `kvcache.dir`, `kvcache.max_gb`). Checked end to end with the real `lhd`:

| Server | First reply after the core and the server were restarted, 7,090-token chat | Cache |
|---|---|---|
| Plain llama-server, LFM2.5-2.6B | 2.59 s (reads all 7,090 tokens) | off |
| Plain llama-server, LFM2.5-2.6B | 0.12 s (7,055 tokens reused) | on: file 118 MB, restore 0.06 s, mode 0600, removed when the chat was deleted |
| llama-swap with the router's slotkeeper, Qwen3.6-35B, 4,195-token chat | 57.9 s | off |
| llama-swap with the router's slotkeeper, Qwen3.6-35B, 4,195-token chat | 57.5 s | on: saved and restored fine (111 MB, 0.14 s), but **no gain** |

Why there is no gain behind slotkeeper: on every chat request slotkeeper first puts its own state into the slot (it restores a prefix snapshot, then prefills the whole prompt again to keep its single "session" file), so a slot the core restored is overwritten before the model sees the request. Slotkeeper keeps one conversation warm; that is what the core's per-chat files would add, but only a server that leaves the slot alone can use them.

What to do with that (a recommendation): leave `kvcache.dir` empty for router entries that run through slotkeeper (the feature is then off and costs nothing), and set it for entries that run plain llama-server. Or teach slotkeeper to keep one session file per conversation, which gives the same result inside the router; that tool lives in `~/AI/rig` and is not part of this repository.

Other things the end-to-end run showed: a restore request is what makes llama-swap load a model that is not loaded (15 s for Qwen here), so the restore time in the log includes the load; the file lands in a per-model folder below `kvcache.dir` and the core finds it there; the activity log has sizes and times only.

## After a tree move: keep the saved file and let the server trim? (2026-10-12)

Question: today a move in the chat tree deletes the saved cache file, so the next reply reads the whole branch again. Could the core keep the old leaf's file, restore it, and send the branch prompt, so the server keeps the shared start and reads only the rest?

Setup: plain `llama-server` build b11541 (`~/AI/engines/_prebuilt/mainline-b11541`), one slot, `--slot-save-path`, no router and no slotkeeper. `branch_trim.py` builds an old chat and a branch that share the first 80 percent (5,582 of about 7,150 tokens, and 25,806 of about 32,850), then measures with the server's own timings. Raw results are in `trim-*.json`. Another process was using 2.4 GB of the GPU during the runs, so the runs used the CPU (`-ngl 0`); a first 7k run on the GPU gave the same token counts and the same shape of result. Everything stayed on this machine.

### A. A model with attention layers only (MiniCPM5-2B, architecture `llama`)

| | Tokens read again | Prompt time | Notes |
|---|---|---|---|
| 7k, cold read of the branch | 7,154 | 2.6 s on GPU, 4.6 s on CPU | |
| 7k, restore old file, then send the branch | 1,572 | 0.7 s on GPU, 1.3 s on CPU | restore itself 0.07 s; 5,582 tokens reused, exactly the shared start |
| 32k, cold read of the branch | 32,849 | 35.1 s on CPU | |
| 32k, restore old file, then send the branch | 7,043 | 9.5 s on CPU | restore itself 0.27 s; 25,806 reused |

So the theory holds for a plain attention model: the server compares the restored tokens with the new prompt, keeps the common start and removes the rest. Sending the old prompt again after a restore reads 1 token.

### B. Qwen3.6-35B (architecture `qwen35moe`), run bare, KV q8_0

The GGUF metadata says `qwen35moe.full_attention_interval = 4` and has `ssm.*` keys (conv kernel 4, state size 128, inner size 4096): three of every four layers are linear-attention (recurrent) layers. They keep one running state, not one entry per token, so the server cannot cut the cache back to an earlier token. Result (7,143-token branch): cold read 63.6 s, restore-then-send 62.7 to 71.3 s, 0 tokens reused. The server log line (also with `-cms 1024`):

`forcing full prompt re-processing due to lack of cache data (likely due to SWA or hybrid/recurrent memory, see https://github.com/ggml-org/llama.cpp/pull/13194#issuecomment-2868343055)`

before it, the log shows `checking checkpoint with [7055, 7055] against 5582...` and the same for 6927: the slot file does carry context checkpoints (two, 62.8 MiB each, saved and restored with the file), but they sit at the end of the old prompt, after the point where the branch begins, so none can be used. `--checkpoint-min-step` only limits how close checkpoints may be; it does not make periodic ones, so it did not help.

What does work on this model: a file made from the shared start alone. Reading the 5,582-token start, saving it, restoring it, and sending the branch read 1,565 tokens in 17.1 s instead of 63 s (5,578 reused). The server only extends the restored text here and never has to cut it, which a recurrent model can do. That means one file per tree node that has children, written when the branch point is known (or when a node gets a second child), not one file per leaf.

### C. Is the answer the same? Close, not identical

At temperature 0 the restore-then-send reply is not always word for word the reply of the cold read. On the attention model both gave the same facts (code 82022, note 566) but one wrote ", and the note number" where the other wrote " and the note number"; the log-probabilities differ by up to 0.55 at that near-tie, and by 1.5 on the 32k chat. A control shows this is noise from the batch layout, not from the restore: a cold read with `-ub 500` instead of the default 512 differs from the default cold read by the same 0.58. Reading the same prompt twice the same way gives identical output. On Qwen with the prefix file, the cold reply began a thinking section and the restored one skipped it; the restored answer (code 47402, note 549) is correct for the branch. So: equal in meaning, not bit for bit. Nothing was found that gave a wrong answer.

### D. Disk

- Attention model: about 43 KB per token. 7k tokens: 309 MB. 32k tokens: 1.41 GB. Six full 7k branches would hit the 5 GB cap; three 32k branches would.
- Qwen3.6 (this build, KV q8_0): 275 MB at 7k, of which 126 MB is the two checkpoints (the earlier 32k measurement was 404 MiB). A prefix-only file at 5.6k was 259 MB.
- A file per leaf is the expensive way. A file per branch point plus oldest-first removal keeps the cap workable.

### The slot API takes no range

`POST /slots/{id}?action=save|restore|erase` only takes a `filename` (checked in `tools/server/server-context.cpp`, and the build's behaviour matched). There is no way to save or load a token range over HTTP. The server does the trimming itself, only when the model allows it.

### Recommendation (not a decision)

1. For models with attention layers only: keep the file and let the server trim. After a tree move, do not delete the old leaf's file; restore it before the next reply and send the branch prompt with `cache_prompt` on. It needs no new storage format. The saving is the shared part (here 78 to 80 percent of the tokens) and costs under 0.3 s to restore.
2. For hybrid or recurrent models such as Qwen3.6: a restored leaf file does nothing for a branch, so keep deleting it (as now) or do not restore it. If the saving matters, write a file for the branch point itself (the shared start) when a node gets its second child: 17 s instead of 63 s in the test. It costs one more file of about 260 MB per branch point.
3. Do not build a chunk store. The server offers no range save or load, and a file per branch point gives the same saving on hybrid models.
4. The core must know the model type to choose between 1 and 2, or simply try: after a restore, read `cache_n` in the reply timings and delete the file if it was 0.
5. Keep the 5 GB cap and oldest-first removal, and count a file per branch point against it.

lh-core was not changed.
