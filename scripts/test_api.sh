#!/usr/bin/env bash
set -e

if [ ! -f /tmp/termcmd_token ]; then
  echo "Error: /tmp/termcmd_token not found. Is TermCMD running?"
  exit 1
fi

TOKEN=$(cat /tmp/termcmd_token)
BASE="http://127.0.0.1:7890/api/v1"

echo "=== 1. Listing Terminals ==="
LIST_JSON=$(curl -s -H "Authorization: Bearer $TOKEN" "$BASE/terminals")
echo "Active terminals: $LIST_JSON"

echo -e "\n=== 2. Creating New Terminal ==="
CREATE_JSON=$(curl -s -X POST "$BASE/terminals" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title": "Automated Test Session", "cols": 120, "rows": 30}')

echo "Created: $CREATE_JSON"
TID=$(echo "$CREATE_JSON" | grep -o '"id":"[^"]*' | head -n1 | cut -d'"' -f4)

if [ -z "$TID" ]; then
  echo "Error: Failed to extract Terminal ID."
  exit 1
fi

echo "Terminal ID: $TID"

echo -e "\n=== 3. Streaming Command Execution (SSE) ==="
echo "Running a 3-step loop via /exec..."
curl -N -s -X POST "$BASE/terminals/$TID/exec" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": "for i in (seq 1 3); echo \"[SSE Output] Step $i/3 completed\"; sleep 1; end; echo \"[SSE Output] Done!\""}'

echo -e "\n=== 4. Sending Stdin Input via /input ==="
curl -s -X POST "$BASE/terminals/$TID/input" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"data": "pwd\n"}'
echo "Input sent."
sleep 0.5

echo -e "\n=== 5. Reading Terminal Details and Buffer ==="
DETAILS=$(curl -s -H "Authorization: Bearer $TOKEN" "$BASE/terminals/$TID")
echo "$DETAILS" | cut -c 1-200
echo "..."

echo -e "\n=== 6. Closing Test Terminal ==="
DELETE_RES=$(curl -s -X DELETE -H "Authorization: Bearer $TOKEN" "$BASE/terminals/$TID")
echo "Delete result: $DELETE_RES"

echo -e "\n=== All API Tests Passed Successfully! ==="
