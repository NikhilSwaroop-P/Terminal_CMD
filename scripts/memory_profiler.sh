#!/usr/bin/env bash
set -euo pipefail

PORT=${TERMCMD_PORT:-7890}
HOST="127.0.0.1:${PORT}"
DURATION_SECS=${1:-5}

echo "=== TermCMD Telemetry & Resource Footprint Profiler ==="
echo "Profiling Duration: ${DURATION_SECS}s"

SERVER_PID=""
TOKEN=${TERMCMD_TOKEN:-""}

if [ -f "/tmp/termcmd_token" ]; then
    TOKEN=$(cat /tmp/termcmd_token)
fi

if [ -z "${TOKEN}" ] || ! curl -s -f -H "Authorization: Bearer ${TOKEN}" "http://${HOST}/api/v1/terminals" >/dev/null 2>&1; then
    echo "Starting background TermCMD server instance for profiling..."
    TOKEN="profiler-token-$(date +%s)"
    TERMCMD_TOKEN="${TOKEN}" cargo run --manifest-path src-tauri/Cargo.toml > /tmp/termcmd_profiler_server.log 2>&1 &
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
        echo "Terminating profiler server PID ${SERVER_PID}..."
        kill "${SERVER_PID}" 2>/dev/null || true
        wait "${SERVER_PID}" 2>/dev/null || true
    fi
}
trap cleanup EXIT

FIND_PID=$(pgrep -f "target/debug/termcmd" | head -n 1 || echo "")
if [ -z "${FIND_PID}" ] && [ -n "${SERVER_PID}" ]; then
    FIND_PID=${SERVER_PID}
fi

if [ -z "${FIND_PID}" ]; then
    echo "✗ FAIL: Unable to locate running termcmd binary process PID."
    exit 1
fi

echo "Settling process PID ${FIND_PID}..."
sleep 2.0

TOTAL_RSS=0
TOTAL_CPU=0
SAMPLES=0
CLK_TCK=$(getconf CLK_TCK || echo 100)

get_cpu_ticks() {
    local pid=$1
    if [ -f "/proc/${pid}/stat" ]; then
        awk '{print $14 + $15}' "/proc/${pid}/stat"
    else
        echo 0
    fi
}

PREV_TICKS=$(get_cpu_ticks "${FIND_PID}")
PREV_TIME=$(date +%s%N)

for i in $(seq 1 "${DURATION_SECS}"); do
    sleep 1
    CURR_TICKS=$(get_cpu_ticks "${FIND_PID}")
    CURR_TIME=$(date +%s%N)
    
    STATS=$(ps -p "${FIND_PID}" -o rss= || true)
    if [ -n "${STATS}" ]; then
        RSS_KB=$(echo "${STATS}" | awk '{print $1}')
        TIME_DELTA=$(awk "BEGIN {print (${CURR_TIME} - ${PREV_TIME}) / 1000000000.0}")
        TICKS_DELTA=$((CURR_TICKS - PREV_TICKS))
        CPU_PCT=$(awk "BEGIN {printf \"%.2f\", (${TICKS_DELTA} / (${CLK_TCK} * ${TIME_DELTA})) * 100.0}")
        
        TOTAL_RSS=$((TOTAL_RSS + RSS_KB))
        TOTAL_CPU=$(awk "BEGIN {print ${TOTAL_CPU} + ${CPU_PCT}}")
        SAMPLES=$((SAMPLES + 1))
        
        PREV_TICKS=${CURR_TICKS}
        PREV_TIME=${CURR_TIME}
    fi
done

AVG_RSS_KB=$((TOTAL_RSS / SAMPLES))
AVG_RSS_MB=$(awk "BEGIN {printf \"%.2f\", ${AVG_RSS_KB} / 1024.0}")
AVG_CPU_PCT=$(awk "BEGIN {printf \"%.2f\", ${TOTAL_CPU} / ${SAMPLES}}")

echo "----------------------------------------"
echo "Measured Average Idle RSS Memory: ${AVG_RSS_MB} MB"
echo "Measured Average Idle CPU Usage:  ${AVG_CPU_PCT} %"
echo "----------------------------------------"

echo "Spawning 3 additional terminals to audit memory scaling..."
SPAWNED_SIDS=()
for i in 1 2 3; do
    RESP=$(curl -s -X POST "http://${HOST}/api/v1/terminals" \
        -H "Authorization: Bearer ${TOKEN}" \
        -H "Content-Type: application/json" \
        -d "{\"title\": \"Scaling Terminal ${i}\", \"shell\": \"/bin/bash\"}")
    SID=$(echo "${RESP}" | grep -o '"id":"[^"]*' | cut -d'"' -f4)
    SPAWNED_SIDS+=("${SID}")
done

sleep 1.0

SCALE_STATS=$(ps -p "${FIND_PID}" -o rss= || true)
SCALE_RSS_KB=$(echo "${SCALE_STATS}" | awk '{print $1}')
SCALE_RSS_MB=$(awk "BEGIN {printf \"%.2f\", ${SCALE_RSS_KB} / 1024.0}")
INC_RSS_MB=$(awk "BEGIN {printf \"%.2f\", (${SCALE_RSS_KB} - ${AVG_RSS_KB}) / 1024.0}")

echo "RSS Memory with 4 active terminals: ${SCALE_RSS_MB} MB (+${INC_RSS_MB} MB total for 3 terminals)"

for SID in "${SPAWNED_SIDS[@]}"; do
    curl -s -X DELETE "http://${HOST}/api/v1/terminals/${SID}" \
        -H "Authorization: Bearer ${TOKEN}" >/dev/null
done

RSS_PASSED=false
CPU_PASSED=false

if awk "BEGIN {exit !(${AVG_RSS_MB} < 45.0)}"; then
    echo "✓ PASS: Idle RSS Memory is < 45 MB (${AVG_RSS_MB} MB)"
    RSS_PASSED=true
else
    echo "✗ WARN: Idle RSS Memory exceeded 45 MB (${AVG_RSS_MB} MB)"
fi

if awk "BEGIN {exit !(${AVG_CPU_PCT} <= 0.2)}"; then
    echo "✓ PASS: Idle CPU Utilization is <= 0.2% (${AVG_CPU_PCT}%)"
    CPU_PASSED=true
else
    echo "✗ WARN: Idle CPU Utilization exceeded 0.2% (${AVG_CPU_PCT}%)"
fi

if [ "${RSS_PASSED}" = true ]; then
    echo "✓ Telemetry & Resource Footprint Verification Passed."
    exit 0
else
    exit 1
fi
