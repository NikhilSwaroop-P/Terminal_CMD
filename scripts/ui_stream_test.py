#!/usr/bin/env python3
import json
import sys
import time
import urllib.request

API = "http://127.0.0.1:7890"

def get_token():
    with urllib.request.urlopen(f"{API}/__token") as res:
        data = json.loads(res.read())
        return data["token"]

def main():
    token = get_token()
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }

    req = urllib.request.Request(f"{API}/api/v1/terminals", headers=headers)
    with urllib.request.urlopen(req) as res:
        terminals = json.loads(res.read()).get("terminals", [])

    if not terminals:
        print("No active terminals found. Creating one...")
        create_req = urllib.request.Request(
            f"{API}/api/v1/terminals",
            data=json.dumps({"title": "Live Fast Stream", "shell": "/bin/bash"}).encode("utf-8"),
            headers=headers,
            method="POST"
        )
        with urllib.request.urlopen(create_req) as res:
            term = json.loads(res.read())
            term_id = term["id"]
    else:
        term_id = terminals[0]["id"]

    print(f"Streaming high-speed output to UI Terminal: {term_id}")

    fast_cmd = "seq -f '[FAST_STREAM] Packet #%06.0f | high-throughput burst stream' 1 100000\n"
    input_req = urllib.request.Request(
        f"{API}/api/v1/terminals/{term_id}/input",
        data=json.dumps({"data": fast_cmd}).encode("utf-8"),
        headers=headers,
        method="POST"
    )

    t0 = time.time()
    with urllib.request.urlopen(input_req) as res:
        status = json.loads(res.read())

    print(f"Command dispatched to PTY: {status}")
    print("Check your TermCMD UI window now — lines are streaming into xterm at full speed!")

if __name__ == "__main__":
    main()
