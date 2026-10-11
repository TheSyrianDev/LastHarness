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
