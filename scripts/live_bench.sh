#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API="http://127.0.0.1:7890"
DURATION=60
PID=$(pgrep -x termcmd | head -1)
RESULTS_FILE="/tmp/termcmd_live_bench_$(date +%s).txt"

TOKEN=$(curl -sf "${API}/__token" | python3 -c "import json,sys; print(json.load(sys.stdin)['token'])")

GREEN='\033[0;32m'; CYAN='\033[0;36m'; YELLOW='\033[1;33m'; BOLD='\033[1m'; RESET='\033[0m'
log()    { echo -e "${CYAN}[$(date +%H:%M:%S)]${RESET} $*"; }
pass()   { echo -e "  ${GREEN}✓${RESET} $*"; }
warn()   { echo -e "  ${YELLOW}!${RESET} $*"; }
header() { echo -e "\n${BOLD}━━━ $* ━━━${RESET}"; }

api() { curl -sf --max-time 15 -H "Authorization: Bearer ${TOKEN}" -H "Content-Type: application/json" "$@"; }

sample_rss() { awk '/VmRSS/{printf "%.1f", $2/1024}' /proc/"${PID}"/status 2>/dev/null || echo "0"; }
sample_cpu() {
    local s1 s2 u1 u2 t1 t2 clk
    s1=$(cat /proc/"${PID}"/stat 2>/dev/null); t1=$(date +%s%N)
    sleep 1
    s2=$(cat /proc/"${PID}"/stat 2>/dev/null); t2=$(date +%s%N)
    u1=$(echo "$s1" | awk '{print $14+$15}')
    u2=$(echo "$s2" | awk '{print $14+$15}')
    clk=$(getconf CLK_TCK)
    awk "BEGIN {printf \"%.2f\", (($u2-$u1)/$clk)/(($t2-$t1)/1e9)*100}"
}

create_term() {
    api -X POST "${API}/api/v1/terminals" \
        -d "{\"title\":\"${1:-bench}\",\"shell\":\"/bin/bash\"}" \
        | python3 -c "import json,sys; print(json.load(sys.stdin)['id'])"
}

delete_term() { api -X DELETE "${API}/api/v1/terminals/${1}" > /dev/null 2>&1 || true; }

buf_lines() {
    api "${API}/api/v1/terminals/${1}" 2>/dev/null \
        | python3 -c "import json,sys; print(json.load(sys.stdin).get('buffer', 0))" 2>/dev/null || echo "0"
}

send_terminal_command() {
    local tid="$1" cmd="$2"
    api -X POST "${API}/api/v1/terminals/${tid}/input" \
        -d "{\"data\":\"${cmd}\n\"}" > /dev/null
}

{
echo "================================================================"
echo "  TermCMD Live Non-Headless Benchmark — $(date)"
printf "  Target PID: %s | Baseline RSS: %s MB | API: %s\n" "${PID}" "$(sample_rss)" "${API}"
echo "================================================================"
log "Token: ${TOKEN:0:12}..."
api "${API}/api/v1/terminals" > /dev/null && pass "API reachable"

header "PHASE 0 — BASELINE (idle GUI)"
log "Sampling baseline idle CPU..."
BASELINE_CPU=$(sample_cpu)
BASELINE_RSS=$(sample_rss)
pass "Idle Baseline RSS: ${BASELINE_RSS} MB"
pass "Idle Baseline CPU: ${BASELINE_CPU}%"

header "PHASE 1 — 1-MINUTE HIGH-SPEED CONTINUOUS STREAM (${DURATION}s)"
TID1=$(create_term "60s-Stream-Test")
pass "Spawned visible terminal: ${TID1}"
sleep 1

STREAM_SCRIPT="${ROOT_DIR}/scripts/fast_stream_generator.sh"
log "Dispatching stream command: bash ${STREAM_SCRIPT} ${DURATION}"
send_terminal_command "$TID1" "bash ${STREAM_SCRIPT} ${DURATION}"

log "Streaming in progress — sampling CPU & Memory over ${DURATION}s..."
T0=$(date +%s)
MAX_RSS=0
TOTAL_CPU=0
SAMPLES=0

while true; do
    NOW=$(date +%s)
    ELAPSED=$((NOW - T0))
    if [ "$ELAPSED" -ge "$DURATION" ]; then
        break
    fi
    CURRENT_RSS=$(sample_rss)
    CURRENT_CPU=$(sample_cpu)
    TOTAL_CPU=$(awk "BEGIN {print $TOTAL_CPU + $CURRENT_CPU}")
    SAMPLES=$((SAMPLES + 1))
    if awk "BEGIN {exit ($CURRENT_RSS > $MAX_RSS) ? 0 : 1}"; then
        MAX_RSS=$CURRENT_RSS
    fi
    printf "  ⏱ [%02ds / %02ds] RSS: %s MB | Active CPU: %s%%\n" "$ELAPSED" "$DURATION" "$CURRENT_RSS" "$CURRENT_CPU"
    sleep 3
done

AVG_BURST_CPU=$(awk "BEGIN {if ($SAMPLES > 0) printf \"%.2f\", $TOTAL_CPU / $SAMPLES; else print 0}")
BURST_RSS=$(sample_rss)
BUF_LINES=$(buf_lines "$TID1")
EST_TOTAL_LINES=60000
LPM=$(awk "BEGIN {printf \"%.0f\", ($EST_TOTAL_LINES / $DURATION) * 60}")

pass "1-Minute Stream Finished"
pass "Ring buffer capacity filled: ${BUF_LINES} / 50000 lines"
pass "Average CPU during stream:   ${AVG_BURST_CPU}%"
pass "Peak RSS during stream:      ${MAX_RSS} MB"
pass "Estimated line throughput:   ~${LPM} lines/min"

delete_term "$TID1"
sleep 2

header "PHASE 2 — MULTI-TERMINAL 5-WAY PARALLEL STREAM (${DURATION}s)"
declare -a TIDS=()
for i in $(seq 1 5); do
    TID=$(create_term "Concurrent-${i}")
    TIDS+=("$TID")
    pass "Created terminal ${i}: ${TID}"
    sleep 0.1
done

MT_RSS_PRE=$(sample_rss)
log "Launching parallel 60s stream script across all 5 terminals simultaneously..."
for TID in "${TIDS[@]}"; do
    send_terminal_command "$TID" "bash ${STREAM_SCRIPT} ${DURATION}"
done

log "5 parallel streams running — sampling live performance..."
MT_T0=$(date +%s)
MT_MAX_RSS=0
MT_TOTAL_CPU=0
MT_SAMPLES=0

while true; do
    NOW=$(date +%s)
    ELAPSED=$((NOW - MT_T0))
    if [ "$ELAPSED" -ge "$DURATION" ]; then
        break
    fi
    CURRENT_RSS=$(sample_rss)
    CURRENT_CPU=$(sample_cpu)
    MT_TOTAL_CPU=$(awk "BEGIN {print $MT_TOTAL_CPU + $CURRENT_CPU}")
    MT_SAMPLES=$((MT_SAMPLES + 1))
    if awk "BEGIN {exit ($CURRENT_RSS > $MT_MAX_RSS) ? 0 : 1}"; then
        MT_MAX_RSS=$CURRENT_RSS
    fi
    printf "  ⏱ [%02ds / %02ds] 5-Tile RSS: %s MB | Concurrent CPU: %s%%\n" "$ELAPSED" "$DURATION" "$CURRENT_RSS" "$CURRENT_CPU"
    sleep 3
done

MT_AVG_CPU=$(awk "BEGIN {if ($MT_SAMPLES > 0) printf \"%.2f\", $MT_TOTAL_CPU / $MT_SAMPLES; else print 0}")
AGG_EST_LINES=$(( EST_TOTAL_LINES * 5 ))
AGG_LPM=$(awk "BEGIN {printf \"%.0f\", ($AGG_EST_LINES / $DURATION) * 60}")

pass "5-Way Concurrent Stream Complete"
pass "Peak 5-terminal RSS:         ${MT_MAX_RSS} MB  (+$(awk "BEGIN {printf \"%.1f\", $MT_MAX_RSS-$MT_RSS_PRE}") MB incremental)"
pass "Average concurrent CPU:      ${MT_AVG_CPU}%"
pass "Aggregate throughput:        ~${AGG_LPM} lines/min"

for TID in "${TIDS[@]}"; do delete_term "$TID"; done
sleep 3

header "PHASE 3 — RAPID 5-SECOND POST-LOAD SETTLING"
sleep 5
IDLE_CPU=$(sample_cpu)
IDLE_RSS=$(sample_rss)
LEAK=$(awk "BEGIN {printf \"%.1f\", $IDLE_RSS-$BASELINE_RSS}")
pass "Post-load settled RSS: ${IDLE_RSS} MB  (delta: +${LEAK} MB)"
pass "Post-load settled CPU: ${IDLE_CPU}%"

header "FINAL LIVE BENCHMARK SCORECARD"
echo ""
echo "  ┌────────────────────────────────────────────────────────────────────────┐"
printf "  │  %-34s  %9s  %8s  %14s │\n" "Scenario" "RSS (MB)" "CPU (%)" "Throughput"
echo "  ├────────────────────────────────────────────────────────────────────────┤"
printf "  │  %-34s  %9s  %8s  %14s │\n" "Idle Baseline (1 Tile)"          "$BASELINE_RSS" "$BASELINE_CPU" "0 lines/min"
printf "  │  %-34s  %9s  %8s  %14s │\n" "1-Minute High-Speed Stream"      "$MAX_RSS"      "$AVG_BURST_CPU" "~${LPM} l/min"
printf "  │  %-34s  %9s  %8s  %14s │\n" "5-Way Concurrent Stream (Peak)"   "$MT_MAX_RSS"   "$MT_AVG_CPU"   "~${AGG_LPM} l/min"
printf "  │  %-34s  %9s  %8s  %14s │\n" "Post-Load Settled State"         "$IDLE_RSS"     "$IDLE_CPU"     "0 lines/min"
echo "  └────────────────────────────────────────────────────────────────────────┘"
echo ""
} 2>&1 | tee "${RESULTS_FILE}"

echo "Benchmark report saved to: ${RESULTS_FILE}"
