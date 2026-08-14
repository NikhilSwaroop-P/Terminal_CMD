# TermCMD: Phase 1 Architecture, Design & Master Execution Plan

---

## 1. Project Overview & Problem Statement

**TermCMD** is a high-performance desktop terminal canvas and orchestration manager designed specifically for AI agent workflows and human-in-the-loop terminal observability.

### Key Problems Solved
1. **Loss of Session State**: Traditional agent subshells (`sh -c`) discard working directories, environment variables, and background processes between tool calls. TermCMD maintains persistent pseudo-terminals (PTYs).
2. **Observability & Intervention**: Operators gain full visibility into executing commands on a GPU-accelerated desktop canvas and can interact directly (type keystrokes, send signals, or scroll output).
3. **Robust Semantic Boundaries**: Replaces fragile wrapper scripts with standard **OSC 133** semantic prompt escape sequences and **OSC 7** working directory synchronization.
4. **Token Optimization**: Dual-stream architecture provides clean, stripped plaintext for AI agents (saving tokens) while rendering rich WebGL syntax-highlighted ANSI in the UI.

---

## 2. Phase 1 Accomplishments & Current State (Source of Truth)

Phase 1 established the formal architectural foundation, visual design, mathematical layout engine, and API wire protocols. All specifications are persisted in:
- **Design Specification**: [`docs/DESIGN_DOC.md`](../docs/DESIGN_DOC.md)
- **Vector SVG Mockups**: [`docs/mockups/`](../docs/mockups/)
- **Milestone Tracker**: [`plans/PLAN_TRACKER.md`](PLAN_TRACKER.md)

### Detailed Inventory of Completed Artifacts:

1. **System Architecture Decomposition**:
   - Asynchronous Rust backend runtime using `Tokio`, `Axum`, and `portable-pty`.
   - Threading and I/O pipeline: Non-blocking slave PTY reader loops, 50,000-line circular ring buffers, broadcast channels.
   - Loopback security model (`127.0.0.1:7890`) with bearer token authentication.

2. **Semantic Shell Hooks (OSC 133 & OSC 7)**:
   - Non-destructive Bash (`PROMPT_COMMAND` + debug trap) and Zsh (`add-zsh-hook precmd/preexec`) integration hooks for zero-mangling command exit code capture and live CWD tracking.

3. **Agent Local API & Dual-Stream Wire Protocols**:
   - Complete OpenAPI-compliant REST schemas for `/api/v1/terminals` (lifecycle, resize, signals, inspection).
   - SSE streaming endpoint (`/api/v1/terminals/:id/exec`) supporting `stripAnsi: true`, interactive prompt detection (`event: prompt_waiting`), execution timeouts, and concurrency guards (`409 Conflict`).

4. **Desktop Canvas & Window Management Specifications**:
   - Dynamic tiling grid with configurable columns per row (`1`, `2`, `3`, `4` cols) and infinite vertical scrolling.
   - Top-insertion rule for newly spawned terminal windows.
   - Native direct edge and corner hover resizing (`↔`, `↕`, `⤡`) with dynamic adjacent tile reflow.
   - Linux standard top-right window controls (`_ □ ✕`).
   - Collapsible sidebar session tree with double-click to pin/move to top and instant remove buttons.
   - Three sorting & arrangement modes: Running Priority, Recent Activity (MRU), Creation Order.

5. **Kitty Aesthetics & WebGL Cursor Trailing Shader**:
   - Dark Tokyo Night / Obsidian palette with high-contrast syntax tokens.
   - Spring-damper physics formulation ($m\mathbf{p}'' + c\mathbf{p}' + k\Delta\mathbf{p} = 0$) with exponential glow falloff for smooth animated cursor trailing.

6. **Verified Vector SVG Mockups**:
   - `mockup_a_sidebar_expanded.svg`: 2-column tiling with edge hover resizing and expanded session sidebar.
   - `mockup_b_grid_tiles.svg`: 3-column priority grid layout.
   - `mockup_c_stacked_zen.svg`: 1-column wide stack mode.

---

## 3. Implementation Roadmap (Phases 2 through 6)

### Phase 2: Tauri 2.0 & Rust Core PTY Engine
- Initialize `src-tauri` with Cargo dependencies (`portable-pty`, `tokio`, `axum`, `serde`, `tower-http`).
- Build PTY session supervisor managing master/slave pairs, asynchronous read/write streams, and process group signals (`SIGINT`, `SIGTERM`, `SIGKILL`).
- Implement zero-allocation byte scanner state machine for OSC 133 command delimiters and OSC 7 directory paths.
- Implement in-memory 50,000-line circular ring buffer for each terminal instance.

### Phase 3: Embedded Local Agent API & Dual Streaming
- Setup Axum HTTP server on loopback (`127.0.0.1:7890`) protected by session bearer tokens.
- Implement REST endpoints for session provisioning, listing, resizing, and signal dispatching.
- Implement Server-Sent Events (SSE) streaming engine with `stripAnsi` token optimization and interactive prompt idle detection.

### Phase 4: Desktop Canvas UI & Kitty Visuals
- Scaffold Vite + TypeScript frontend.
- Implement responsive dynamic tiling engine with infinite vertical scroll and edge/corner drag-resizing.
- Build collapsible sidebar session organizer with sorting state machine (Running Priority, MRU, Creation).
- Integrate `xterm.js` with WebGL addon and custom Kitty spring-physics cursor trailing shader overlay.
- Implement top-right Linux window controls (`_ □ ✕`), minimized dock tray, and copy-on-select clipboard integration.

### Phase 5: Testing, Verification & Performance Benchmarking
- Execute end-to-end integration test suite.
- Run high-volume stress benchmark ($>100,000\text{ lines/min}$) verifying zero UI freezes.
- Verify Linux platform compatibility (Wayland / X11) and idle memory footprint ($<45\text{MB}$).

### Phase 6: Model Context Protocol (MCP) Integration (Final Milestone)
- Implement embedded JSON-RPC stdio/SSE MCP server adapter (`termcmd --mcp`).
- Expose standard MCP tools (`termcmd_create_terminal`, `termcmd_run_command`, `termcmd_send_input`, `termcmd_list_terminals`).
- Verify out-of-the-box compatibility with MCP-compatible agent clients.

---

## 4. Verification & Testing Framework

Once implementation begins, the following automated and manual test matrix will be executed to validate system correctness:

### 4.1 Automated Unit & Integration Tests
| Test Suite | Target Component | Validation Criteria |
| :--- | :--- | :--- |
| `test_pty_spawn_and_lifecycle` | Rust PTY Manager | Verifies clean process fork, initial prompt emit, and teardown. |
| `test_osc133_prompt_detection` | OSC Parser | Verifies accurate identification of `A`, `B`, `C`, and `D;<exit_code>` markers across Bash/Zsh. |
| `test_osc7_cwd_tracking` | OSC Parser | Verifies directory changes (`cd /path`) update session CWD without delay. |
| `test_sse_stream_chunks` | Axum SSE Server | Verifies stdout chunks stream sequentially and connection closes cleanly on exit code. |
| `test_ansi_stripper` | ANSI Filter | Verifies raw escape sequences are completely stripped for agent consumers. |
| `test_sigint_interruption` | Process Supervisor | Verifies `SIGINT` interrupts active subprocess without killing the parent shell. |
| `test_tiling_reflow_math` | Canvas Engine | Verifies dragging border $i$ adjusts tile $i$ and $i+1$ without exceeding boundary constraints. |

### 4.2 Stress & Performance Verification
- **Log Burst Test**: Stream 100,000 lines from `/dev/urandom` through PTY, verifying zero UI thread deadlocks and stable frame rates ($>55\text{ FPS}$).
- **Concurrency Test**: 10 simultaneous agent sessions executing parallel build jobs while validating prompt tracking accuracy.
- **Resource Footprint Audit**: Confirm idle RSS is $<45\text{MB}$ and idle CPU utilization is $<0.2\%$.
