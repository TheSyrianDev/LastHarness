# Spike 1: tool-call compatibility on llama-swap (port 9292)

Status: llama-swap run done for all 7 models. The Ollama or LM Studio run and the cloud API run are not done (the cloud run needs the user's go-ahead).

How to repeat: `./run.sh <model>`. Raw captures are in `captures/<model>/`, timings in `run.log`. Server build seen: llama.cpp `b287-a97cce8`. `max_tokens` was 1024 in every request.

Models: LFM2.5-2.6B-fast, MiniCPM5-2B-max, Spark-X2.5-4B-fast, Ornith-1.5-35B-A3B-NanoPlus-fast, Qwen3.6-35B-A3B-NanoPlus-fast, Tiel-Coder-35B-fast, SignOfFour-fast. Tests: plain chat, streamed chat, tool call, streamed tool call, tool-result round trip, two tools at once, `tool_choice: none`, `tool_choice` forced to a named tool.

## Short answer

Native tool calls work on all 7 models, streamed or not. The decision to use native tool calls only, with no prompted fallback, holds. The server's chat template does the work, as expected.

What does not work reliably, and the harness must handle:
1. Thinking is always in `reasoning_content`, and it can eat the whole token budget.
2. `tool_choice: "none"` does not stop tool calls. They come out as text in `content`, in a different syntax per model family.
3. A forced `tool_choice` was ignored by every model.

## Table

| Model | Tool call | Streamed call | Round trip | Two tools | `none` honoured | Forced honoured |
|---|---|---|---|---|---|---|
| LFM2.5-2.6B-fast | ok | ok | ok | ok | no, text | no |
| MiniCPM5-2B-max | ok | ok | ok | ok | no, text | no |
| Spark-X2.5-4B-fast | ok | ok | ok | ok | no, text | no |
| Ornith-1.5-35B-A3B-NanoPlus-fast | ok | ok | ok | ok | no, text (wrong tool name) | no (ran out of tokens) |
| Qwen3.6-35B-A3B-NanoPlus-fast | ok | ok | ok | ok | no, text | no (ran out of tokens) |
| Tiel-Coder-35B-fast | ok | ok | ok | ok | no, text | no |
| SignOfFour-fast | ok | ok | ok | ok | no, text | no |

## What was the same on every model

- Tool call: `finish_reason: "tool_calls"`, `arguments` is a JSON string that parses when complete, `id` is present (random string), `content` is `""`.
- Streamed call: the first chunk has `index`, `id`, `type` and the function name. Later chunks carry only `index` and an `arguments` fragment. The fragments join into valid JSON. The last delta is empty with `finish_reason: "tool_calls"`. With `stream_options.include_usage`, `usage` arrives in its own final chunk with `choices: []`.
- Two tools in one turn came back as two tool calls with separate ids.
- Round trip: every model answered from the tool result with no further tool call.
- Thinking arrives in `reasoning_content`, in both streamed and non-streamed replies. No model put thinking tags inside `content`.
- Responses carry a non-standard `timings` object on the last chunk, and the `model` field is the gguf file path, not the name that was asked for.
- The prefix cache works (`cached_tokens` is non-zero on repeated prefixes).

## Where models differ

- **Thinking eats the budget.** On a trivial prompt ("say hello in five words"), MiniCPM5-2B-max, Spark-X2.5-4B-fast and Qwen3.6-35B-A3B-NanoPlus-fast hit the 1024-token limit while still thinking, so `content` was empty and `finish_reason` was `length`. The others finished (115 to 595 tokens). The harness needs a generous token limit for thinking models and must handle an empty `content` with `finish_reason: "length"` clearly.
- **Text-form tool calls under `tool_choice: none`.** Each family writes its own syntax into `content`:
  - LFM: `<|tool_call_start|>[get_weather(city='Paris')]<|tool_call_end|>`
  - MiniCPM: `<function name="get_weather"><param name="city">Paris</param></function>`
  - Spark: `<tool_call>get_weather<arg_key>city</arg_key><arg_value>Paris</arg_value></tool_call>`
  - Ornith, Qwen, Tiel, SignOfFour: `<tool_call><function=get_weather><parameter=city>Paris</parameter></function></tool_call>`
  - Ornith even invented a tool name (`weather` instead of `get_weather`).
  
  So `none` cannot be trusted. If the harness wants no tool use, it should leave the tools out of the request instead.
- **Forced tool choice.** No model made the named call. Five answered the greeting in plain text; Ornith and Qwen ran out of tokens while thinking (`finish_reason: "length"`, empty `content`). The harness should not rely on forcing.
- **Speed (rough, from `run.log`).** A plain tool call took 0.7 to 1.4 s on the three small models and 5 to 10 s on the 35B models. First request on each model includes load time (22 to 60 s for chat test 01).

## What this means for the compatibility table (Milestone 1)

- Always read `reasoning_content`, and accept `reasoning` as well (this server only used the first, so the second is from the spec, not verified here).
- Join `arguments` fragments by `index` and parse once at the end of the call.
- Treat `usage` as optional and possibly in a chunk with empty `choices`.
- Ignore unknown fields (`timings`, `system_fingerprint`).
- Do not send `tool_choice: none` or a forced choice and expect it to work on this server. Remove the tools instead.
- Set a high enough `max_tokens` for thinking models, and report a `length` finish with empty content as a clear error.

## Not covered

- Variants other than the seven named models were not run, since nothing looked odd enough to need it.
- One prompt per test. Reliability over many tries (a model sometimes not calling a tool when it should) is not measured.
- Ollama, LM Studio and a cloud API were not tested, so none of this says how those servers differ.
