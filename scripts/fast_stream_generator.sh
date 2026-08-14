#!/usr/bin/env bash
set -euo pipefail

DURATION="${1:-60}"

python3 -c "
import sys
import time

duration = float(${DURATION})
end_time = time.time() + duration
count = 0

sys.stdout.write(f'=== FAST STREAM GENERATOR STARTED (Target: {duration}s) ===\n')
sys.stdout.flush()

while time.time() < end_time:
    chunk = ''.join([
        f'[{time.strftime(\"%H:%M:%S\")}.{int(time.time()*1000)%1000:03d}] PACKET #{count + i:07d} | HIGH-SPEED PTY STREAM | RATE=MAX\n'
        for i in range(1, 1001)
    ])
    count += 1000
    sys.stdout.write(chunk)
    sys.stdout.flush()
    time.sleep(0.015)

sys.stdout.write(f'=== FAST STREAM GENERATOR FINISHED: {count} lines streamed in {duration}s ===\n')
sys.stdout.flush()
"
