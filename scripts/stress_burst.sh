#!/usr/bin/env bash
set -euo pipefail

PORT=${TERMCMD_PORT:-7890}
HOST="127.0.0.1:${PORT}"
TOKEN=${TERMCMD_TOKEN:-"stress-burst-token-$(date +%s)"}
LINES_COUNT=${1:-100000}

echo "=== TermCMD High-Volume Burst Benchmark ==="
echo "Target lines: ${LINES_COUNT}"
echo "API Host: http://${HOST}"

SERVER_PID=""
if ! curl -s -f "http://${HOST}/api/v1/terminals?token=${TOKEN}" >/dev/null 2>&1; then
    echo "Starting local TermCMD server instance..."
    TERMCMD_TOKEN="${TOKEN}" cargo run --manifest-path src-tauri/Cargo.toml > /tmp/termcmd_burst_server.log 2>&1 &
    SERVER_PID=$!
    
    for i in $(seq 1 30); do
        if [ -f "/tmp/termcmd_token" ]; then
            TOKEN=$(cat /tmp/termcmd_token)
        fi
        if curl -s -H "Authorization: Bearer ${TOKEN}" "http://${HOST}/api/v1/terminals" >/dev/null 2>&1; then
            echo "TermCMD server is ready."
            break
        fi
        sleep 0.2
    done
fi

cleanup() {
    if [ -n "${SERVER_PID}" ]; then
        echo "Terminating benchmark server PID ${SERVER_PID}..."
        kill "${SERVER_PID}" 2>/dev/null || true
        wait "${SERVER_PID}" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo "Spawning benchmark terminal session..."
CREATE_RESP=$(curl -s -X POST "http://${HOST}/api/v1/terminals" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{"title": "Burst Stress Session", "cols": 120, "rows": 40}')

TERM_ID=$(echo "${CREATE_RESP}" | grep -o '"id":"[^"]*' | cut -d'"' -f4)
echo "Spawned terminal ID: ${TERM_ID}"

sleep 1.0

echo "Streaming ${LINES_COUNT} lines through PTY pipeline..."
START_TS=$(date +%s%N)

EXEC_RESP=$(curl -s -N -X POST "http://${HOST}/api/v1/terminals/${TERM_ID}/exec" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{\"command\": \"seq 1 ${LINES_COUNT}\", \"stripAnsi\": true, \"timeoutSeconds\": 30}")

END_TS=$(date +%s%N)
NANOS=$((END_TS - START_TS))
MILLIS=$((NANOS / 1000000))
SECONDS=$(awk "BEGIN {print ${MILLIS} / 1000.0}")
LPM=$(awk "BEGIN {print (${LINES_COUNT} / (${MILLIS} / 1000.0)) * 60.0}")

echo "Burst execution completed in: ${SECONDS}s (${MILLIS}ms)"
printf "Throughput: %.2f lines/min\n" "${LPM}"

EXIT_CODE=$(echo "${EXEC_RESP}" | grep -o '"exitCode":[0-9]*' | head -n 1 | cut -d':' -f2 || echo "0")
echo "Reported exit code: ${EXIT_CODE}"

echo "Fetching terminal buffer snapshot..."
INSPECT_RESP=$(curl -s -X GET "http://${HOST}/api/v1/terminals/${TERM_ID}" \
    -H "Authorization: Bearer ${TOKEN}")

CURL_STATUS=$(echo "${INSPECT_RESP}" | grep -o '"state":{"type":"[^"]*' | cut -d'"' -f5 || echo "Idle")
echo "Terminal state after burst: ${CURL_STATUS}"

curl -s -X DELETE "http://${HOST}/api/v1/terminals/${TERM_ID}" \
    -H "Authorization: Bearer ${TOKEN}" >/dev/null

if awk "BEGIN {exit !(${LPM} >= 100000.0)}"; then
    echo "✓ PASS: Burst throughput exceeded target (>100,000 lines/min)."
    exit 0
else
    echo "✗ FAIL: Burst throughput was below target."
    exit 1
fi
