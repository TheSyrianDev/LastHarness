#!/usr/bin/env bash
# Spike 1: tool-call compatibility on llama-swap. Throwaway. curl + jq only.
# Usage: ./run.sh <model> [base_url]
set -u
MODEL="$1"
BASE="${2:-http://localhost:9292}"
DIR="$(cd "$(dirname "$0")" && pwd)/captures/$MODEL"
mkdir -p "$DIR"
URL="$BASE/v1/chat/completions"

WEATHER='{"type":"function","function":{"name":"get_weather","description":"Get current weather for a city","parameters":{"type":"object","properties":{"city":{"type":"string"}},"required":["city"]}}}'
TIME='{"type":"function","function":{"name":"get_time","description":"Get current local time for a city","parameters":{"type":"object","properties":{"city":{"type":"string"}},"required":["city"]}}}'

# post <name> <stream:true|false> <json body>
post() {
  local name="$1" body="$3"
  local out="$DIR/$name"
  [ "$2" = true ] && out="$out.sse" || out="$out.json"
  local t0=$(date +%s.%N)
  curl -sS -m 600 -N -o "$out" -w '%{http_code}' "$URL" -H 'Content-Type: application/json' -d "$body" > "$DIR/$name.status" 2> "$DIR/$name.err"
  local t1=$(date +%s.%N)
  printf '%s status=%s secs=%.1f\n' "$name" "$(cat "$DIR/$name.status")" "$(echo "$t1 - $t0" | bc)"
}

msg() { jq -cn --arg m "$MODEL" --argjson s "$1" --argjson msgs "$2" --argjson tools "$3" --argjson extra "${4:-{\}}" \
  '{model:$m, stream:$s, max_tokens:1024, messages:$msgs} + (if ($tools|length)>0 then {tools:$tools} else {} end) + $extra'; }

U1='[{"role":"user","content":"Say hello in five words."}]'
U2='[{"role":"user","content":"What is the weather in Paris? Use the tool."}]'
U3='[{"role":"user","content":"What is the weather and the local time in Tokyo? Use the tools."}]'

post 01-chat        false "$(msg false "$U1" '[]')"
post 02-chat-stream true  "$(msg true  "$U1" '[]' '{"stream_options":{"include_usage":true}}')"
post 03-tool        false "$(msg false "$U2" "[$WEATHER]")"
post 04-tool-stream true  "$(msg true  "$U2" "[$WEATHER]" '{"stream_options":{"include_usage":true}}')"

# 05 round trip: reuse the call from test 03 if it produced one, else a fake call.
CALL=$(jq -c '.choices[0].message.tool_calls[0] // empty' "$DIR/03-tool.json" 2>/dev/null)
if [ -z "$CALL" ]; then
  CALL='{"id":"call_fake1","type":"function","function":{"name":"get_weather","arguments":"{\"city\":\"Paris\"}"}}'
  echo "05: no tool call in 03, using fake call" > "$DIR/05-note.txt"
fi
CID=$(echo "$CALL" | jq -r '.id // "call_fake1"')
RT=$(jq -cn --argjson u "$U2" --argjson c "$CALL" --arg id "$CID" \
  '$u + [{"role":"assistant","content":null,"tool_calls":[$c]},{"role":"tool","tool_call_id":$id,"content":"{\"temp_c\":18,\"sky\":\"cloudy\"}"}]')
post 05-roundtrip   false "$(msg false "$RT" "[$WEATHER]")"

post 06-parallel    false "$(msg false "$U3" "[$WEATHER,$TIME]")"
post 07-choice-none false "$(msg false "$U2" "[$WEATHER]" '{"tool_choice":"none"}')"
post 08-choice-force false "$(msg false "$U1" "[$WEATHER]" '{"tool_choice":{"type":"function","function":{"name":"get_weather"}}}')"
