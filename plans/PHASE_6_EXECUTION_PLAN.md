# TermCMD: Phase 6 Agent CLI Tool, Live Stream Pipe & Universal Agent Skill Plan

---

## 1. Phase 6 Objectives & Scope

Phase 6 delivers the **Direct Agent CLI & Live Streaming Bridge (`termcmd`)** along with a **Universal AI Agent Skill (`SKILL.md`)**. 

While REST/SSE endpoints exist, AI coding assistants, background agents, and shell scripts need a frictionless, zero-configuration command-line interface that requires no manual header setup or token parsing. 

The CLI and skill are engineered to be **100% distribution-agnostic across Linux** (Debian, Ubuntu, Fedora, Arch, RHEL, openSUSE, Alpine, NixOS) and **compatible with ANY AI agent framework** (Antigravity, Claude Code, Cline, Roo Code, Cursor, OpenCode, Aider, LangChain/LlamaIndex agents, or custom LLM tool callers).

```mermaid
flowchart TB
    subgraph MultiAgentEcosystem ["Universal AI Agent Ecosystem"]
        ClaudeCode["Claude Code / Anthropic"]
        Antigravity["Antigravity / Gemini"]
        CursorCline["Cursor / Cline / Roo Code"]
        OpenCodeAider["OpenCode / Aider / Shell"]
        UniversalSkill["Universal Agent Skill (SKILL.md)"]
    end

    subgraph UniversalCLI ["termcmd Universal Linux CLI Binary"]
        TokenResolver["XDG & Fallback Token / Port Resolver"]
        StreamEngine["Real-Time SSE Streamer -> stdout"]
        SignalForwarder["POSIX SIGINT / SIGTERM Forwarder"]
    end

    subgraph CrossDistroLinux ["Cross-Distro Linux Discovery (XDG / POSIX)"]
        XDGRuntime["$XDG_RUNTIME_DIR/termcmd.token"]
        XDGConfig["$XDG_CONFIG_HOME/termcmd/token"]
        TmpFallback["/tmp/termcmd-$UID/token"]
        PortSpec["$XDG_RUNTIME_DIR/termcmd.port (Default: 7890)"]
    end

    subgraph RunningTermCMD ["Running TermCMD Application (Tauri 2.0 Core)"]
        APIServer["Axum API & SSE Stream Engine"]
        PTYSession["Persistent PTY Process (Bash/Zsh/Fish/Sh)"]
    end

    UniversalSkill -->|Loaded by| ClaudeCode & Antigravity & CursorCline & OpenCodeAider
    ClaudeCode & Antigravity & CursorCline & OpenCodeAider -->|Runs termcmd CLI| UniversalCLI
    UniversalCLI -->|Resolves paths| CrossDistroLinux
    UniversalCLI -->|SSE / REST / HTTP| APIServer
    APIServer <-->|Pipes Output & Signals| PTYSession
    APIServer -->|Live SSE Chunks (event: stdout)| UniversalCLI
    UniversalCLI -->|Real-Time Stdout Stream| ClaudeCode & Antigravity & CursorCline & OpenCodeAider
```

---

## 2. Universal Linux Compatibility Architecture

The CLI and backend adhere strictly to standard Linux specifications:
1. **XDG Base Directory Specification**:
   - Token search priority:
     1. `$TERMCMD_TOKEN` environment variable.
     2. `$XDG_RUNTIME_DIR/termcmd.token` (systemd / elogind runtime dir, e.g. `/run/user/1000/termcmd.token`).
     3. `$XDG_CONFIG_HOME/termcmd/token` or `~/.config/termcmd/token`.
     4. `/tmp/termcmd-$UID/token` (fallback on minimal Linux systems without systemd, e.g. Alpine/Void).
   - Port search priority:
     1. `$TERMCMD_PORT` environment variable.
     2. `$XDG_RUNTIME_DIR/termcmd.port` or `~/.config/termcmd/port`.
     3. Fallback default: `7890`.
2. **Shell Discovery & Fallback**:
   - Automatically detects user's `$SHELL` or searches `PATH` for standard binaries (`/bin/bash`, `/usr/bin/bash`, `/bin/zsh`, `/usr/bin/zsh`, `/usr/bin/fish`, `/bin/sh`).
3. **Zero External Runtime Dependencies**:
   - The CLI compiles to a standalone statically/dynamically linked native Linux binary without glibc version locks, distribution-specific system libraries, or Python/Node runtime requirements.

---

## 3. Universal Agent Skill Specification

The skill is defined in `skills/termcmd/SKILL.md` using the industry-standard YAML frontmatter and markdown documentation recognized by all modern agentic assistants:

- **YAML Frontmatter**:
  - `name: termcmd`
  - `description`: Multi-terminal orchestration, live stream command execution, and prompt interaction via the `termcmd` CLI.
- **Universal Agent Patterns**:
  - Step-by-step guidance for agents on how to list, spawn, stream-execute, answer prompts, and kill terminals.
  - Formatted so any agent with a bash/shell execution tool (`run_command`, `bash`, `execute_command`, `terminal`) can utilize TermCMD without modifications.

---

## 4. CLI Subcommand Suite

| Subcommand | Arguments & Options | Universal Behavior & Description |
| :--- | :--- | :--- |
| `spawn` | `[--title <name>] [--cwd <path>] [--shell <shell>] [--cols <c>] [--rows <r>]` | Spawns a persistent terminal on the canvas; outputs only the new `id` to stdout for clean agent scripting. |
| `list` | `[--json]` | Lists open terminals in a human-readable table or clean JSON for agent parsing. |
| `exec` | `<id> "<command>" [--raw] [--timeout <secs>] [--env K=V...]` | Dispatches command to PTY, **streams stdout chunks live to agent console in real-time**, and exits with the subprocess exit code ($0$ for success, non-zero for error). |
| `input` | `<id> "<data>"` | Forwards raw stdin data or interactive answers (`[y/N]`, password, text) to the PTY. |
| `kill` | `<id> [--signal SIGINT\|SIGTERM\|SIGKILL]` | Sends POSIX signal to foreground process group. |
| `close` | `<id>` | Closes the shell and removes the terminal tile from the canvas. |
| `snapshot` | `<id> [--lines <n>]` | Fetches recent buffer history from the 50,000-line ring buffer. |
| `resize` | `<id> --cols <c> --rows <r>` | Updates PTY kernel dimensions via `SIGWINCH`. |

---

## 5. Verification & Testing Plan for Phase 6

### 5.1 Automated CLI Integration Tests (`src-tauri/tests/cli_tests.rs`)
1. **`test_cli_cross_distro_token_discovery`**:
   - Tests token resolution via env, XDG runtime dir, config dir, and `/tmp/termcmd-$UID/` fallback.
2. **`test_cli_spawn_and_list`**:
   - Validates `termcmd spawn` creates session and `termcmd list --json` returns valid JSON.
3. **`test_cli_exec_streaming_output`**:
   - Executes `termcmd exec <id> "echo 'Live Streaming Line'"` and captures streaming stdout chunks.
4. **`test_cli_exit_code_propagation`**:
   - Executes `termcmd exec <id> "sh -c 'exit 42'"` and verifies CLI process exits with code `42`.
5. **`test_cli_interactive_input`**:
   - Executes `read -p 'Name: ' name && echo "Hello $name"`, sends `termcmd input <id> "World"`, and asserts `"Hello World"` in output.
6. **`test_cli_sigint_forwarding`**:
   - Executes `sleep 30`, sends `SIGINT` via `termcmd kill`, and verifies clean early termination.

### 5.2 Multi-Agent Verification
- Test skill instructions with various LLM prompt formats and agent tooling environments.

