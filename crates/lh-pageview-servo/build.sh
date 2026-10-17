#!/bin/sh
# Build the Servo page helper. It is large: the first build takes a long time and a lot of disk.
# Keep it to 4 jobs on a shared machine. The result is target/release/lh-pageview-servo; put it next to the app.
set -e
cd "$(dirname "$0")"
nice -n 10 cargo build --release -j "${JOBS:-4}"
