import urllib.request
import json
import time

def run():
    token = open("/tmp/termcmd_token").read().strip()
    base = "http://127.0.0.1:7890/api/v1"
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}

    def request(method, path, data=None):
        body = json.dumps(data).encode("utf-8") if data else None
        req = urllib.request.Request(f"{base}{path}", data=body, headers=headers, method=method)
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read().decode("utf-8"))

    print("=== 1. Spawning new terminal ===")
    status, created = request("POST", "/terminals", {"title": "Python API Test", "cols": 100, "rows": 28})
    tid = created["id"]
    pid = created["pid"]
    print(f"Created Terminal: {tid} (PID: {pid})")
    time.sleep(1.5)

    print("\n=== 2. Streaming execution via SSE /exec ===")
    stream_req = urllib.request.Request(
        f"{base}/terminals/{tid}/exec",
        data=json.dumps({"command": "for i in (seq 1 4); echo \"[Task] Working on item $i/4...\"; sleep 1; end; echo \"[Task] Completed!\""}).encode("utf-8"),
        headers=headers,
        method="POST"
    )
    with urllib.request.urlopen(stream_req) as stream:
        for line in stream:
            decoded = line.decode("utf-8").strip()
            if decoded:
                print(f"  {decoded}")

    print("\n=== 3. Sending input command ===")
    request("POST", f"/terminals/{tid}/input", {"data": "echo \"Hello from TermCMD API\"\n"})
    time.sleep(0.5)

    print("\n=== 4. Fetching terminal details ===")
    status, details = request("GET", f"/terminals/{tid}")
    term_info = details["terminal"]
    print(f"ID: {term_info['id']}, CWD: {term_info['cwd']}, Shell: {term_info['shell']}")

    print("\n=== 5. Closing terminal ===")
    request("DELETE", f"/terminals/{tid}")
    print("Closed successfully.")

if __name__ == "__main__":
    run()
