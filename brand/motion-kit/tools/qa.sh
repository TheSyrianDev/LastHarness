#!/usr/bin/env bash
# QA for out/lottie: each segment except spawn (which starts from nothing) must have pixel-identical first and last frames (clean cuts and loops),
# and contact sheets per file go to out/qa. Needs: node + playwright-core + lottie-web next to render_v1.js
# (NODE_DIR), a Chromium binary (CHROME), ImageMagick.
set -u; K="$(cd "$(dirname "$0")/.." && pwd)"; T="$(mktemp -d)"; fail=0
for f in "$K"/out/lottie/*-bg.json; do n=$(basename "$f" .json)
  segs=$(python3 -c "import json,sys;[print(m['cm'],m['tm'],m['tm']+m['dr']) for m in json.load(open(sys.argv[1]))['markers'] if ':' not in m['cm'] and m['cm']!='spawn']" "$f")
  frames=$(echo "$segs" | awk '{print $2","$3}' | paste -sd, | tr ',' '\n' | sort -nu | paste -sd,)
  node "$NODE_DIR/render_v1.js" "$f" "$T/$n" 128 "$frames" >/dev/null
  while read s a b; do d=$(magick compare -fuzz 1% -metric AE "$T/$n/f$(printf %04d $a).png" "$T/$n/f$(printf %04d $b).png" null: 2>&1 | cut -d' ' -f1)
    d=${d%.*}; [ "${d:-0}" = 0 ] && r=ok || { r="DIFF $d px"; fail=1; }; echo "$n $s $a-$b $r"; done <<< "$segs"
  node "$NODE_DIR/render_v1.js" "$f" "$T/$n-sheet" 96 0:99999:12 >/dev/null
  magick montage "$T/$n-sheet"/*.png -tile 16x -geometry +1+1 -background '#333' "$K/out/qa/$n-sheet.png"
done; rm -rf "$T"; exit $fail
