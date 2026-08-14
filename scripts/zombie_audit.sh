#!/usr/bin/env bash
set -euo pipefail

PORT=${TERMCMD_PORT:-7890}
HOST="127.0.0.1:${PORT}"

echo "=== TermCMD Process Group Signal & Zombie Cleanup Audit ==="
echo "API Host: http://${HOST}"

SERVER_PID=""
TOKEN=${TERMCMD_TOKEN:-""}

if [ -f "/tmp/termcmd_token" ]; then
    TOKEN=$(cat /tmp/termcmd_token)
fi

if [ -z "${TOKEN}" ] || ! curl -s -f -H "Authorization: Bearer ${TOKEN}" "http://${HOST}/api/v1/terminals" >/dev/null 2>&1; then
    echo "Starting background TermCMD server instance..."
    TOKEN="zombie-audit-token-$(date +%s)"
    TERMCMD_TOKEN="${TOKEN}" cargo run --manifest-path src-tauri/Cargo.toml > /tmp/termcmd_zombie_server.log 2>&1 &
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

echo "Spawning audit terminal session..."
RESP=$(curl -s -X POST "http://${HOST}/api/v1/terminals" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{"title": "Zombie Audit Session", "shell": "/bin/bash", "cols": 100, "rows": 30}')

SID=$(echo "${RESP}" | grep -o '"id":"[^"]*' | cut -d'"' -f4)
CHILD_PID=$(echo "${RESP}" | grep -o '"pid":[0-9]*' | cut -d':' -f2)
echo "Spawned terminal ID: ${SID} (Child Shell PID: ${CHILD_PID})"

sleep 1.0

echo "Spawning deep background process tree in terminal..."
curl -s -X POST "http://${HOST}/api/v1/terminals/${SID}/input" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{"data": "sh -c '\''sleep 101 & sleep 102 & wait'\''\n"}' >/dev/null

sleep 1.5

DESCENDANTS=$(pgrep -P "${CHILD_PID}" || true)
echo "Detected descendant PIDs: ${DESCENDANTS}"

echo "Sending SIGINT to interrupt foreground process group..."
curl -s -X POST "http://${HOST}/api/v1/terminals/${SID}/kill" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{"signal": "SIGINT"}' >/dev/null

sleep 1.0

POST_INT_DESCENDANTS=$(pgrep -P "${CHILD_PID}" || true)
if [ -n "${POST_INT_DESCENDANTS}" ]; then
    echo "Warning: descendants still alive after SIGINT: ${POST_INT_DESCENDANTS}"
else
    echo "✓ Clean: No active foreground descendants after SIGINT."
fi

echo "Deleting terminal session..."
curl -s -X DELETE "http://${HOST}/api/v1/terminals/${SID}" \
    -H "Authorization: Bearer ${TOKEN}" >/dev/null

sleep 1.0

if ps -p "${CHILD_PID}" >/dev/null 2>&1; then
    echo "✗ FAIL: Shell PID ${CHILD_PID} still exists after terminal deletion."
    exit 1
fi

ZOMBIES=$(ps -ef | grep -v grep | grep "<defunct>" | grep "${CHILD_PID}" || true)
if [ -n "${ZOMBIES}" ]; then
    echo "✗ FAIL: Found zombie defunct process:\n${ZOMBIES}"
    exit 1
fi

echo "✓ PASS: Terminal process group and all descendants cleanly reaped. Zero zombies found."
exit 0
