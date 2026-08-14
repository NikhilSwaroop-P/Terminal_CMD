# TermCMD: Phase 6 Agent CLI Tool, Live Stream Pipe & Agent Skill Plan

---

## 1. Phase 6 Objectives & Scope

Phase 6 delivers the **Direct Agent CLI & Live Streaming Bridge (`termcmd`)** along with an **Antigravity Agent Skill**. While standard REST/SSE endpoints exist, AI coding assistants, background agents, and shell scripts need a frictionless, zero-configuration command-line interface that requires no manual header setup or token parsing. 

The CLI connects directly to the running TermCMD backend, streaming terminal output chunk-by-chunk to the agent's standard output in real-time as if the command were executing directly in their local terminal, and exiting with the exact subprocess exit code upon completion.

```mermaid
flowchart TB
    subgraph AgentEnvironment ["AI Agent / Shell Environment"]
        Agent["AI Coding Agent / User Terminal"]
        AgentSkill["TermCMD Agent Skill (SKILL.md)"]
        CLI["termcmd CLI Binary (Rust)"]
    end

    subgraph AuthDiscovery ["Zero-Config Discovery"]
        TokenFile["$XDG_RUNTIME_DIR/termcmd.token"]
        PortFile["$XDG_RUNTIME_DIR/termcmd.port (Default: 7890)"]
    end

    subgraph BackendEngine ["TermCMD Application (Tauri + Axum + PTY)"]
        APIServer["Axum API Server"]
        PTYSession["Persistent PTY Process (Bash/Zsh/Fish)"]
    end

    AgentSkill -->|Directs Agent to use| CLI
    Agent -->|termcmd spawn / exec / input / list| CLI
    CLI -->|Auto-reads Token & Port| AuthDiscovery
    CLI -->|SSE / REST / HTTP| APIServer
    APIServer <-->|Pipes Output & Signals| PTYSession
    APIServer -->|Live SSE Chunks (event: stdout)| CLI
    CLI -->|Real-Time Stdout Stream| Agent
```

---

## 2. Detailed Work Breakdown Structure (WBS)

### Sub-Milestone 6.1: Rust Agent CLI Binary (`termcmd`)
- **Binary Target**: Add a high-performance, standalone CLI binary target (`src-tauri/src/bin/termcmd.rs` or `crates/termcmd-cli`) compiled alongside the main application.
- **Instant Cold Startup ($<3\text{ms}$)**: Uses `clap` (derive parser) and `reqwest` / `ureq` / `tokio` for lightweight execution.
- **Zero-Config Auth & Port Resolution**:
  - Checks `TERMCMD_TOKEN` environment variable.
  - Falls back to reading token from `$XDG_RUNTIME_DIR/termcmd.token` or `~/.config/termcmd/token`.
  - Resolves server port from `$TERMCMD_PORT`, `$XDG_RUNTIME_DIR/termcmd.port`, or default `7890`.
  - If TermCMD is not running or unreachable, emits a clean actionable error message with exit code `1`.

### Sub-Milestone 6.2: CLI Subcommand Suite

| Subcommand | Arguments & Options | Behavior & Description |
| :--- | :--- | :--- |
| `spawn` | `[--title <name>] [--cwd <path>] [--shell <shell>] [--cols <c>] [--rows <r>]` | Spawns a new persistent terminal on the canvas and outputs the new `id`. |
| `list` | `[--json]` | Lists all open terminals, showing ID, Title, State (`IDLE`/`RUNNING`), CWD, and active PID. |
| `exec` | `<id> "<command>" [--raw] [--timeout <secs>] [--env K=V...]` | Dispatches command to persistent PTY, **streams stdout/stderr chunks live to agent console**, and returns subprocess exit code. |
| `input` | `<id> "<data>"` | Forwards raw stdin data or interactive response (`[y/N]`, text) to the PTY. |
| `kill` | `<id> [--signal SIGINT\|SIGTERM\|SIGKILL]` | Sends POSIX signal to the foreground process group. |
| `close` | `<id>` | Closes the shell and removes the terminal tile from the canvas. |
| `snapshot` | `<id> [--lines <n>]` | Fetches recent buffer history from the 50,000-line ring buffer. |
| `resize` | `<id> --cols <c> --rows <r>` | Updates PTY kernel dimensions. |

### Sub-Milestone 6.3: Real-Time SSE Stream Forwarding & Signal Passthrough
- **Chunk-by-Chunk Real-Time Streaming**:
  - Connects to `POST /api/v1/terminals/:id/exec` with `stripAnsi: true` (or `--raw` for ANSI color).
  - Immediately writes each received `event: stdout` chunk to `std::io::stdout()`, flushing after each chunk.
  - Enables agents to observe progress (e.g. build logs, downloads, test progress) before the command completes.
- **Interactive Prompt Detection**:
  - When `event: prompt_waiting` is received, the CLI outputs a distinct notice: `[termcmd] Waiting for interactive input... (use 'termcmd input <id> "<response>"')`.
- **Exit Code Propagation**:
  - When `event: done` is received, extracts `exitCode` and terminates the CLI with `std::process::exit(exit_code)`.
- **POSIX Signal Forwarding (`Ctrl+C`)**:
  - Traps `SIGINT` in the CLI process and immediately sends `POST /api/v1/terminals/:id/kill` (`SIGINT`) to the running backend process group.

### Sub-Milestone 6.4: TermCMD Antigravity Agent Skill (`skills/termcmd/SKILL.md`)
- **Skill Specification**:
  - Created at `skills/termcmd/SKILL.md` and installed to the system skill directory (`~/.gemini/config/skills/termcmd/SKILL.md`).
  - Documents exact usage patterns for LLMs:
    1. **Inspecting open terminals**: `termcmd list`
    2. **Spawning project workspaces**: `termcmd spawn --title "Backend Server" --cwd /path/to/project`
    3. **Running commands with live progress**: `termcmd exec <id> "cargo test"`
    4. **Handling interactive confirmation prompts**: Detecting prompt notices and answering via `termcmd input <id> "yes"`.
    5. **Interrupting stuck processes**: `termcmd kill <id> --signal SIGINT`.

---

## 3. Directory Layout for Phase 6 Deliverables

```
Terminal_CMD/
├── src-tauri/
│   └── src/
│       └── bin/
│           └── termcmd.rs          <-- Standalone Rust CLI binary
├── skills/
│   └── termcmd/
│       └── SKILL.md                <-- Agent Skill specification
├── scripts/
│   └── install_cli.sh              <-- Symlinks/installs termcmd binary to PATH
└── tests/
    └── cli_tests.rs                <-- CLI integration test suite
```

---

## 4. Verification & Testing Plan for Phase 6

### 4.1 Automated CLI Integration Tests (`tests/cli_tests.rs`)
1. **`test_cli_token_and_port_discovery`**:
   - Validates that the CLI automatically detects the running server token and loopback port.
2. **`test_cli_spawn_and_list`**:
   - Runs `termcmd spawn --title "Test"` and validates session creation in `termcmd list`.
3. **`test_cli_exec_streaming_output`**:
   - Executes `termcmd exec <id> "echo 'Live Streaming Line'"` and captures live stdout.
4. **`test_cli_exit_code_propagation`**:
   - Executes `termcmd exec <id> "sh -c 'exit 42'"` and verifies CLI process exits with code `42`.
5. **`test_cli_interactive_input`**:
   - Executes `read -p 'Name: ' name && echo "Hello $name"`, sends `termcmd input <id> "World"`, and asserts `"Hello World"` in output.
6. **`test_cli_sigint_forwarding`**:
   - Executes `sleep 30`, sends `SIGINT` via `termcmd kill`, and verifies clean early termination.

### 4.2 Agent Skill Verification
- Test running skill workflows from an agent prompt, confirming seamless terminal multiplexing and command execution without authentication hurdles.
