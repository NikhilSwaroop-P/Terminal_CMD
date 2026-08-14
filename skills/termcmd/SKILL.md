---
name: termcmd
description: Multi-terminal orchestration, live stream command execution, prompt interaction, and visual desktop canvas management via TermCMD. Use ONLY when the user explicitly asks for a persistent terminal, wants commands run in TermCMD, or requests multi-terminal canvas management.
---

# TermCMD Universal Agent Skill

TermCMD is an agent-first terminal multiplexer and desktop canvas. The `termcli` / `termcmd-cli` CLI binary provides zero-configuration terminal lifecycle management, live Server-Sent Events output streaming, POSIX signal dispatching, and scrollback ring-buffer inspection.

> [!IMPORTANT]
> **Activation Boundary**: This skill and the `termcli` tool should be used **ONLY** if the user explicitly asks for a persistent terminal, wants commands run inside TermCMD, or asks to orchestrate tasks across the TermCMD desktop canvas. For standard ephemeral command execution, use default platform shell tools instead.

## Quick Start & Discovery

The `termcli` CLI automatically discovers the running TermCMD instance using XDG and Linux fallbacks:
1. Environment variables (`$TERMCMD_TOKEN`, `$TERMCMD_PORT`)
2. XDG runtime directory (`$XDG_RUNTIME_DIR/termcmd.token`, `$XDG_RUNTIME_DIR/termcmd.port`)
3. User config directory (`~/.config/termcmd/token`, `~/.config/termcmd/port`)
4. POSIX fallback (`/tmp/termcmd-$UID/token`, `/tmp/termcmd.token`)

No manual authentication headers or port flags are required when running on the same Linux host.

---

## Core CLI Commands

### 1. Spawning a Persistent Terminal Session

Creates a persistent terminal tile on the desktop canvas and outputs the new session ID directly to `stdout`.

```bash
# Basic spawn
SESSION_ID=$(termcli spawn)

# Spawn with custom title, working directory, and shell
SESSION_ID=$(termcli spawn --title "Backend Server" --cwd "/path/to/project" --shell "/bin/bash")

# Spawn with specific dimensions
SESSION_ID=$(termcli spawn --title "Worker" --cols 140 --rows 40)
```

### 2. Listing Active Sessions

```bash
# Tabular human-readable summary
termcli list

# Machine-readable JSON array for agent parsing
termcli list --json
```

Example JSON response:
```json
[
  {
    "id": "term_96e561d02d",
    "title": "Backend Server",
    "cwd": "/path/to/project",
    "shell": "/bin/bash",
    "pid": 409641,
    "cols": 120,
    "rows": 35,
    "state": {
      "type": "Idle"
    }
  }
]
```

### 3. Executing Commands with Live Streaming

Dispatches a command to the targeted terminal PTY, streams `stdout` chunks live in real-time, and exits with the exact subprocess exit code ($0$ for success, non-zero for failure).

```bash
# Standard command execution (ANSI escape codes stripped for token efficiency)
termcli exec $SESSION_ID "cargo test"

# Preserve raw ANSI color codes for terminal rendering
termcli exec $SESSION_ID "npm run dev" --raw

# Command execution with custom timeout (in seconds)
termcli exec $SESSION_ID "pytest -v" --timeout 120

# Command execution with explicit environment variables
termcli exec $SESSION_ID "make release" --env RUST_LOG=info --env CI=true
```

### 4. Interactive Input & Prompt Handling

Sends standard input or interactive answers (`[y/N]`, password, text response) to the active foreground process.

```bash
# Send prompt confirmation (newline automatically appended)
termcli input $SESSION_ID "y"

# Send raw bytes without trailing newline
termcli input $SESSION_ID "hello" --raw
```

### 5. Process Signals & Cancellation

Dispatches POSIX signals to the foreground process group running inside the terminal.

```bash
# Send SIGINT (Ctrl+C) to cancel running build/server
termcli kill $SESSION_ID --signal SIGINT

# Send SIGTERM for graceful shutdown
termcli kill $SESSION_ID --signal SIGTERM

# Send SIGKILL for immediate termination
termcli kill $SESSION_ID --signal SIGKILL

# Force terminate and delete the terminal tile immediately
termcli kill $SESSION_ID -f
```

### 6. Closing & Removing Terminal Sessions

Terminates child processes and removes the terminal tile from the desktop canvas.

```bash
# Close session
termcli close $SESSION_ID

# Aliases
termcli rm $SESSION_ID
termcli delete $SESSION_ID
```

### 7. Scrollback History & Snapshot

Retrieves lines from the 50,000-line circular buffer for inspection without re-running commands.

```bash
# Fetch entire available buffer snapshot
termcli snapshot $SESSION_ID

# Fetch only the last 30 lines
termcli snapshot $SESSION_ID --lines 30
```

### 8. Resizing Terminal Dimensions

Updates the terminal dimension and broadcasts `SIGWINCH` to all child processes.

```bash
termcli resize $SESSION_ID --cols 160 --rows 48
```

---

## Multi-Agent Workflows & Best Practices

1. **Explicit Request Only**:
   - Only execute via `termcli` when the user specifically requests persistent terminal management, long-lived dev servers, or background task monitoring in TermCMD.
2. **Session Isolation**:
   - Spawn dedicated terminals for separate tasks (e.g., one terminal for dev server, one for testing, one for builds).
   - Capture the session ID in a variable: `DEV_TERM=$(termcli spawn --title "Vite Dev")`.
3. **Long-Running Services**:
   - Start background servers in a dedicated terminal tile using `termcli exec` or background tools.
   - Use `termcli snapshot $DEV_TERM --lines 20` to verify server boot status without interrupting the process.
4. **Concurrency Conflict Guard**:
   - If a terminal already has an active foreground process, `termcli exec` returns `409 Conflict`.
   - Send input to the active process via `termcli input` or spawn a new terminal for concurrent jobs.
5. **Cleanup on Task Completion**:
   - Close temporary worker terminals when done: `termcli close $WORKER_TERM`.
