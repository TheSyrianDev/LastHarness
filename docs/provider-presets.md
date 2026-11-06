# Provider presets

Table: `crates/lh-provider/src/presets.rs`. API: `GET /v1/providers/presets` (list, no keys) and `PUT /v1/provider` with `{"preset": "<id>"}` (fills `provider.url`, `provider.name` and `provider.api`; a key is still entered separately and goes to the secret store). `provider.api` is `openai` (default; chat completions, also Ollama) or `anthropic` (Messages API, `Provider::anthropic` in `crates/lh-provider/src/anthropic.rs`). Anthropic and MiniMax (`https://api.minimax.io/anthropic`, from Pi's source) use `anthropic`. A preset with `supported: false` is listed but cannot be applied; none is unsupported today. `POST /v1/provider/test` takes an optional `api`.

Anthropic path: `POST {base}/v1/messages` with `x-api-key` and `anthropic-version: 2023-06-01`; SSE events `message_start`, `content_block_start/delta/stop`, `message_delta`, `error`. Thinking deltas become reasoning events, `input_json_delta` is joined into tool calls, `cache_read_input_tokens` is `cached_tokens`. `max_tokens` is required, default 8192. Models come from `GET /v1/models` (short built-in list on the official address if that fails). Limits: thinking blocks are not sent back (no signature kept), so extended thinking with tools is unsupported; images are not sent. Wire format checked against Pi's `anthropic-messages.js` and tested with a fake server only; not run against the real API.

## Sources
Pi 1.4.0 provider source (`@earendil-works/pi-ai/dist/providers/*.js`): OpenAI, Anthropic, OpenRouter, Groq, Together (`api.together.ai/v1`), Fireworks (`api.fireworks.ai/inference/v1`), DeepSeek, xAI, Cerebras, Moonshot (ai, cn), Kimi Code, Z.AI (coding plan, global and CN), MiniMax (Anthropic API), Qwen Token Plan (Singapore), NVIDIA, Hugging Face, Baseten, Xiaomi MiMo; env var names from `env-api-keys.js`.
Official docs, fetched 2026-11-05: Gemini (`ai.google.dev/gemini-api/docs/openai`, `.../v1beta/openai`), Mistral (`docs.mistral.ai/api`, `/v1/chat/completions`), Vercel AI Gateway (`/v1`), Together, LM Studio (`localhost:1234/v1`), Ollama (`localhost:11434/v1`). llama-swap/LastEngine: the project default (9292).
Oh-my-Pi README (can1357/oh-my-pi) was used only for its provider list.
Key-page links (`help_url`) come from memory of each service's console and were not fetched. Check them before release.

## Left out, not verified
- Perplexity: docs show `/v1/sonar` and `/v1/agent`, no clear chat-completions base.
- Alibaba DashScope pay-as-you-go, GitHub Models, Azure OpenAI (per-resource URL, needs a resource name and api-version), Amazon Bedrock and Google Vertex (cloud credentials, not a key): no address confirmed. Azure and Bedrock need a different settings shape.
- vLLM (`localhost:8000`) and llama.cpp (`127.0.0.1:8080`): default ports not confirmed from fetched docs. Add once checked.
- Others in Oh-my-Pi (SambaNova, DeepInfra, Novita, Venice, NanoGPT, Kilo, ZenMux, OpenCode, Cloudflare, SiliconFlow, Scaleway, LiteLLM...): no address in Pi source or fetched docs.
- Custom: not a table row; the existing manual URL field is the custom option.

## GUI work needed (`crates/lh-gui/src/pages_ui/models_keys.rs`)
1. Preset dropdown at the top of the provider card, filled from `GET /v1/providers/presets`, local group first, with a "Custom" entry that keeps the manual URL field.
2. Choosing a preset fills the URL and name fields (editable) and shows the note and a "Get a key" link from `help_url`.
3. Show the key field only when `key_required`; show `key_env` as a hint ("or set OPENAI_API_KEY").
4. Show unsupported presets greyed with their note; do not allow Save.
5. Save sends `{preset, key}`; keep the existing Test button and "restart needed" message.
6. Optionally show a "local" or "cloud, costs money" badge from `local`.

## Files, pictures, video and audio

A message can carry files. Send them in the turn request: `POST /v1/sessions/{id}/turns` with
`{"text": "...", "attachments": [{"name": "cat.png", "mime": "image/png", "data": "<base64>"}]}`.
`mime` is optional; the kind is worked out from the type, the name and the first bytes.

What happens to each kind depends on what the model takes (the server's own report when it gives one,
otherwise a cautious table by service and model name; unknown means no):

- Images go as `image_url` data URIs (OpenAI-style, Gemini's compatible endpoint, OpenRouter, llama.cpp) or base64 image blocks (Anthropic). Formats other than PNG, JPEG, GIF and WebP are converted with ffmpeg if it is installed.
- Video goes as `video_url` only to models known to take it. Otherwise up to 8 frames are taken with ffmpeg and sent as pictures, and the user is told. With no ffmpeg, or a model without vision, the turn is refused or the file is replaced by a note, with a plain message.
- Audio goes as `input_audio` (WAV or MP3; other formats are converted with ffmpeg) to models that take audio.
- PDF goes as a file part (OpenAI, OpenRouter) or document block (Anthropic). Elsewhere the text is pulled out with `pdftotext` if installed.
- Text files are put in the message as text, framed as data from the file, not instructions.

Nothing is dropped silently: a "notice" event in the turn stream says what was changed, and the model gets a
short note when a file could not be sent. Caps: 10 files, images 20 MB, audio 25 MB, PDF 32 MB, video 100 MB,
text 256 KB, 96 MB together. Over a cap, the turn is refused with HTTP 413 and a sentence naming the file.
A token estimate for the files (images and text only) is sent as a notice and logged.

Files are saved as they were sent under `attachments/<id>` next to the database. The stored message holds
only a marker with the ids. The activity log holds names, types and sizes, never contents. On later turns the
files stay in the history; when a stored chat is resumed they are reloaded and adapted to the model chosen then.
