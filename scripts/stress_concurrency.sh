#!/usr/bin/env bash
set -euo pipefail

PORT=${TERMCMD_PORT:-7890}
HOST="127.0.0.1:${PORT}"
CONCURRENCY=${1:-10}

echo "=== TermCMD 10x Agent Concurrency Stress Benchmark ==="
echo "Target parallel sessions: ${CONCURRENCY}"
echo "API Host: http://${HOST}"

SERVER_PID=""
TOKEN=${TERMCMD_TOKEN:-""}

if [ -f "/tmp/termcmd_token" ]; then
    TOKEN=$(cat /tmp/termcmd_token)
fi

if [ -z "${TOKEN}" ] || ! curl -s -f -H "Authorization: Bearer ${TOKEN}" "http://${HOST}/api/v1/terminals" >/dev/null 2>&1; then
    echo "Starting background TermCMD server instance..."
    TOKEN="concurrency-benchmark-token-$(date +%s)"
    TERMCMD_TOKEN="${TOKEN}" cargo run --manifest-path src-tauri/Cargo.toml > /tmp/termcmd_concurrency_server.log 2>&1 &
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

declare -a SESSION_IDS=()

echo "Spawning ${CONCURRENCY} parallel bash PTY sessions..."
for i in $(seq 1 "${CONCURRENCY}"); do
    RESP=$(curl -s -X POST "http://${HOST}/api/v1/terminals" \
        -H "Authorization: Bearer ${TOKEN}" \
        -H "Content-Type: application/json" \
        -d "{\"title\": \"Concurrency Agent ${i}\", \"shell\": \"/bin/bash\", \"cols\": 100, \"rows\": 30}")
    
    SID=$(echo "${RESP}" | grep -o '"id":"[^"]*' | cut -d'"' -f4)
    SESSION_IDS+=("${SID}")
done

echo "Spawned ${#SESSION_IDS[@]} active sessions. Waiting for prompt initialization..."
sleep 2.0

PIDS=()
TMPDIR=$(mktemp -d /tmp/termcmd_conc_XXXXXX)

START_TS=$(date +%s%N)
for idx in $(seq 0 $((${CONCURRENCY} - 1))); do
    SID="${SESSION_IDS[$idx]}"
    OUT_FILE="${TMPDIR}/session_${idx}.out"
    
    (
        curl -s -N -X POST "http://${HOST}/api/v1/terminals/${SID}/exec" \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/json" \
            -d "{\"command\": \"echo 'MARKER_SESSION_${idx}' && (exit ${idx})\", \"stripAnsi\": true, \"timeoutSeconds\": 15}" \
            > "${OUT_FILE}" 2>&1
    ) &
    PIDS+=($!)
done

for pid in "${PIDS[@]}"; do
    wait "${pid}"
done

END_TS=$(date +%s%N)
MILLIS=$(( (END_TS - START_TS) / 1000000 ))
echo "All ${CONCURRENCY} parallel commands completed in ${MILLIS}ms"

ALL_PASSED=true
for idx in $(seq 0 $((${CONCURRENCY} - 1))); do
    SID="${SESSION_IDS[$idx]}"
    OUT_FILE="${TMPDIR}/session_${idx}.out"
    CONTENT=$(cat "${OUT_FILE}")
    
    if ! echo "${CONTENT}" | grep -q "MARKER_SESSION_${idx}"; then
        echo "✗ FAIL: Session ${idx} missing output marker."
        ALL_PASSED=false
    fi
    
    if ! echo "${CONTENT}" | grep -q "\"exitCode\":${idx}" && ! echo "${CONTENT}" | grep -q "\"exitCode\": ${idx}"; then
        echo "✗ FAIL: Session ${idx} did not report exitCode ${idx}."
        echo "Output was:"
        echo "${CONTENT}"
        ALL_PASSED=false
    fi
done

for SID in "${SESSION_IDS[@]}"; do
    curl -s -X DELETE "http://${HOST}/api/v1/terminals/${SID}" \
        -H "Authorization: Bearer ${TOKEN}" >/dev/null
done

rm -rf "${TMPDIR}"

if [ "${ALL_PASSED}" = true ]; then
    echo "✓ PASS: 10x Agent Concurrency Stress Test succeeded with 100% exit code accuracy."
    exit 0
else
    echo "✗ FAIL: Concurrency verification failed."
    exit 1
fi
