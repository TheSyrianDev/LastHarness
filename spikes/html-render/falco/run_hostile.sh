#!/bin/sh
ip link set lo up
rm -f hits.log
python3 -I listener.py 8123 hits.log &
LP=$!
sleep 0.5
H=/home/omar/AI/spikes-src/falco-target/release/hr-falco
export FALCO_SANS=/usr/share/fonts/liberation/LiberationSans-Regular.ttf HR_FORCE_SCRIPT=1
$H hostile.html hostile.png > hostile.out 2>&1
$H hostile2.html hostile2.png > hostile2.out 2>&1
sleep 1
kill $LP
