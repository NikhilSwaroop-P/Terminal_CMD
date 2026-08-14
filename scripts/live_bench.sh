#!/usr/bin/env bash
set -euo pipefail

API="http://127.0.0.1:7890"
DURATION=60
PID=$(pgrep -x termcmd | head -1)
RESULTS_FILE="/tmp/termcmd_live_bench_$(date +%s).txt"

TOKEN=$(cat /run/user/"$(id -u)"/termcmd.token 2>/dev/null \
    || cat ~/.config/termcmd/token 2>/dev/null \
    || cat /tmp/termcmd.token 2>/dev/null \
    || echo "")

GREEN='\033[0;32m'; CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'
log()    { echo -e "${CYAN}[$(date +%H:%M:%S)]${RESET} $*"; }
pass()   { echo -e "  ${GREEN}✓${RESET} $*"; }
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

buffer_lines() {
    api "${API}/api/v1/terminals/${1}" 2>/dev/null \
        | python3 -c "import json,sys; print(json.load(sys.stdin).get('buffer', 0))" 2>/dev/null || echo "0"
}

stream_exec_count_lines() {
    local tid="$1" cmd="$2" dur="$3"
    local out_file; out_file=$(mktemp)
    curl -sf --max-time $(( dur + 5 )) \
        -H "Authorization: Bearer ${TOKEN}" \
        -H "Accept: text/event-stream" \
        "${API}/api/v1/terminals/${tid}/exec/stream?command=$(python3 -c "import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))" "${cmd}")&shell=/bin/bash" \
        > "$out_file" 2>/dev/null &
    local cpid=$!
    sleep "$dur"
    kill $cpid 2>/dev/null || true
    wait $cpid 2>/dev/null || true
    local count; count=$(grep -c '^data:' "$out_file" 2>/dev/null || echo "0")
    rm -f "$out_file"
    echo "$count"
}

stream_ws_count_lines() {
    local tid="$1" dur="$2"
    local out_file; out_file=$(mktemp)
    timeout "$dur" curl -sf --no-buffer \
        -H "Authorization: Bearer ${TOKEN}" \
        "${API}/api/v1/terminals/${tid}/stream" \
        > "$out_file" 2>/dev/null || true
    local count; count=$(wc -l < "$out_file" 2>/dev/null || echo "0")
    rm -f "$out_file"
    echo "$count"
}

{
echo "================================================================"
echo "  TermCMD Live GUI Benchmark — $(date)"
printf "  PID: %s | RSS: %s MB | API: %s\n" "${PID}" "$(sample_rss)" "${API}"
echo "================================================================"

[ -z "$TOKEN" ] && { echo "ERROR: No token found."; exit 1; }
log "Token: ${TOKEN:0:12}..."
api "${API}/api/v1/terminals" > /dev/null && pass "API reachable"

header "PHASE 0 — BASELINE (idle GUI)"
log "Sampling idle CPU..."
BASELINE_CPU=$(sample_cpu)
BASELINE_RSS=$(sample_rss)
pass "Idle RSS: ${BASELINE_RSS} MB"
pass "Idle CPU: ${BASELINE_CPU}%"

header "PHASE 1 — STREAM THROUGHPUT (1 terminal, ${DURATION}s)"
TID1=$(create_term "burst-bench")
pass "Terminal: ${TID1}"
sleep 0.5

log "Streaming ${DURATION}s of output (yes | head -n 500000)..."
T0=$(date +%s%N)

OUT1=$(mktemp)
curl -sf --max-time $(( DURATION + 5 )) \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Accept: text/event-stream" \
    --get --data-urlencode "command=yes BENCH_LINE | head -n 500000" \
    --data-urlencode "shell=/bin/bash" \
    "${API}/api/v1/terminals/${TID1}/exec/stream" \
    > "$OUT1" 2>/dev/null &
STREAM_PID=$!

sleep 3
BURST_CPU=$(sample_cpu)
BURST_RSS=$(sample_rss)

wait $STREAM_PID 2>/dev/null || true
T1=$(date +%s%N)

SSE_LINES=$(grep -c '^data:' "$OUT1" 2>/dev/null || echo "0")
BUF_LINES=$(buffer_lines "$TID1")
rm -f "$OUT1"

ELAPSED=$(awk "BEGIN {printf \"%.2f\", ($T1-$T0)/1e9}")
LPM_SSE=$(awk "BEGIN {printf \"%.0f\", ($SSE_LINES/$ELAPSED)*60}")
LPM_BUF=$(awk "BEGIN {printf \"%.0f\", ($BUF_LINES/1)*60/${ELAPSED}*${ELAPSED}}")

pass "SSE events received:            ${SSE_LINES}"
pass "Ring buffer lines:              ${BUF_LINES} / 50000"
pass "Elapsed:                        ${ELAPSED}s"
pass "SSE throughput:                 ${LPM_SSE} lines/min"
pass "RSS under burst:                ${BURST_RSS} MB  (+$(awk "BEGIN {printf \"%.1f\", $BURST_RSS-$BASELINE_RSS}") MB)"
pass "CPU under burst:                ${BURST_CPU}%"

delete_term "$TID1"
sleep 2

header "PHASE 2 — MULTI-TERMINAL (5 concurrent, ${DURATION}s)"
declare -a TIDS=()
for i in $(seq 1 5); do
    TID=$(create_term "mt-${i}")
    TIDS+=("$TID")
    pass "Terminal ${i}: ${TID}"
    sleep 0.15
done

MT_RSS_PRE=$(sample_rss)
MT_T0=$(date +%s%N)

declare -a OUTS=()
for TID in "${TIDS[@]}"; do
    OUT=$(mktemp)
    OUTS+=("$OUT")
    curl -sf --max-time $(( DURATION + 5 )) \
        -H "Authorization: Bearer ${TOKEN}" \
        -H "Accept: text/event-stream" \
        --get --data-urlencode "command=yes MT_LOAD | head -n 100000" \
        --data-urlencode "shell=/bin/bash" \
        "${API}/api/v1/terminals/${TID}/exec/stream" \
        > "$OUT" 2>/dev/null &
done

log "5 concurrent SSE streams running..."
sleep 5
MT_CPU=$(sample_cpu)
MT_RSS_PEAK=$(sample_rss)

log "Waiting for all streams to complete..."
wait
MT_T1=$(date +%s%N)
MT_ELAPSED=$(awk "BEGIN {printf \"%.2f\", ($MT_T1-$MT_T0)/1e9}")

TOTAL_SSE=0
for OUT in "${OUTS[@]}"; do
    LC=$(grep -c '^data:' "$OUT" 2>/dev/null || echo "0")
    TOTAL_SSE=$(( TOTAL_SSE + LC ))
    rm -f "$OUT"
done
AVG_SSE=$(( TOTAL_SSE / 5 ))
AGG_LPM=$(awk "BEGIN {printf \"%.0f\", ($TOTAL_SSE/$MT_ELAPSED)*60}")
PER_TERM_LPM=$(awk "BEGIN {printf \"%.0f\", ($AVG_SSE/$MT_ELAPSED)*60}")

pass "Total SSE events (5 terminals): ${TOTAL_SSE}"
pass "Avg per terminal:               ${AVG_SSE} events"
pass "Per-terminal throughput:        ${PER_TERM_LPM} lines/min"
pass "Aggregate throughput:           ${AGG_LPM} lines/min"
pass "RSS at 5-terminal peak:         ${MT_RSS_PEAK} MB  (+$(awk "BEGIN {printf \"%.1f\", $MT_RSS_PEAK-$MT_RSS_PRE}") MB)"
pass "CPU at 5-terminal load:         ${MT_CPU}%"

for TID in "${TIDS[@]}"; do delete_term "$TID"; done
sleep 2

header "PHASE 3 — IDLE DECAY (${DURATION}s leak check)"
log "Waiting ${DURATION}s for RSS to settle..."
sleep "$DURATION"
IDLE_CPU=$(sample_cpu)
IDLE_RSS=$(sample_rss)
RSS_LEAK=$(awk "BEGIN {printf \"%.1f\", $IDLE_RSS-$BASELINE_RSS}")
pass "Post-load RSS:  ${IDLE_RSS} MB  (delta: +${RSS_LEAK} MB from baseline)"
pass "Post-load CPU:  ${IDLE_CPU}%"
awk "BEGIN {exit ($RSS_LEAK > 15) ? 0 : 1}" && echo "  ⚠ RSS growth >15 MB — possible leak" || pass "Leak check OK"

header "FINAL SUMMARY"
echo ""
echo "  ┌──────────────────────────────────────────────────────────────────────┐"
printf "  │  %-33s  %8s  %7s  %13s  │\n" "Scenario" "RSS(MB)" "CPU(%)" "Lines/min"
echo "  ├──────────────────────────────────────────────────────────────────────┤"
printf "  │  %-33s  %8s  %7s  %13s  │\n" "Idle GUI (1 terminal)"      "$BASELINE_RSS" "$BASELINE_CPU" "-"
printf "  │  %-33s  %8s  %7s  %13s  │\n" "Burst stream (1 terminal)"  "$BURST_RSS"    "$BURST_CPU"    "$LPM_SSE"
printf "  │  %-33s  %8s  %7s  %13s  │\n" "5 concurrent (per-terminal)" "$MT_RSS_PEAK" "$MT_CPU"       "$PER_TERM_LPM"
printf "  │  %-33s  %8s  %7s  %13s  │\n" "5 concurrent (aggregate)"   "$MT_RSS_PEAK"  "$MT_CPU"       "$AGG_LPM"
printf "  │  %-33s  %8s  %7s  %13s  │\n" "Post-load idle"             "$IDLE_RSS"     "$IDLE_CPU"     "-"
echo "  └──────────────────────────────────────────────────────────────────────┘"
echo ""
echo "  Targets: RSS idle <45MB (headless) | CPU idle <0.2% | Throughput >100k lpm"
echo "  Note:  GUI WebView RSS baseline ~100-200MB is WebKitGTK overhead (not a leak)"
echo ""
} 2>&1 | tee "${RESULTS_FILE}"

echo "Full results: ${RESULTS_FILE}"
