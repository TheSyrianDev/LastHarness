#!/usr/bin/env bash
# Headless smoke test: starts lhd on a free port with a temp store, then checks the core answers.
# No GUI, no model needed for these checks. Needs target/debug/lhd (cargo build -p lh-core).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LHD="${LHD:-$ROOT/target/debug/lhd}"
[ -x "$LHD" ] || { echo "missing $LHD; run cargo build -p lh-core"; exit 2; }
WORK="$(mktemp -d)"
PORT="${PORT:-17842}"
TOKEN_FILE="$WORK/token"
trap 'kill "$PID" 2>/dev/null || true; rm -rf "$WORK"' EXIT
"$LHD" --listen "127.0.0.1:$PORT" --token-file "$TOKEN_FILE" --store "$WORK/store.db" --no-team >"$WORK/lhd.log" 2>&1 &
PID=$!
for _ in $(seq 1 50); do
  curl -sf "http://127.0.0.1:$PORT/v1/health" >/dev/null 2>&1 && break
  sleep 0.2
done
check() { local name="$1" path="$2" want="$3"
  local body; body="$(curl -sf -H "Host: 127.0.0.1:$PORT" -H "Authorization: Bearer $(cat "$TOKEN_FILE")" "http://127.0.0.1:$PORT$path")" \
    || { echo "FAIL $name: no answer from $path"; tail -5 "$WORK/lhd.log"; exit 1; }
  grep -q "$want" <<<"$body" || { echo "FAIL $name: $path lacks $want"; exit 1; }
  echo "ok   $name"
}
check "health" /v1/health '"ok":true'
check "status" /v1/status '{'
check "models search needs network or empty" /v1/models/installed '{'
check "team external view" /v1/team/external '"external"'
check "schedules list" /v1/schedules '{'
echo "smoke: all checks passed"
