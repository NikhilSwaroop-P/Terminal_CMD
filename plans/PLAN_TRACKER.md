# TermCMD Master Plan & Milestone Tracker

---

## 1. Project Overview & Source of Truth

**TermCMD** is a specialized Linux desktop terminal canvas and orchestration manager built on Tauri 2.0 (Rust + WebGL TypeScript). It enables AI agents to execute, stream, and persist pseudo-terminal (PTY) sessions via local REST/SSE/WS APIs while providing human operators with a GPU-accelerated, Kitty-styled multi-window desktop canvas.

- **Primary Source of Truth (Design Spec)**: [`docs/DESIGN_DOC.md`](../docs/DESIGN_DOC.md)
- **Visual Mockups**: [`docs/mockups/`](../docs/mockups/)
- **Phase 1 Execution Plan**: [`plans/PHASE_1_EXECUTION_PLAN.md`](PHASE_1_EXECUTION_PLAN.md)
- **Phase 2 Execution Plan**: [`plans/PHASE_2_EXECUTION_PLAN.md`](PHASE_2_EXECUTION_PLAN.md)
- **Phase 3 Execution Plan**: [`plans/PHASE_3_EXECUTION_PLAN.md`](PHASE_3_EXECUTION_PLAN.md)
- **Phase 4 Execution Plan**: [`plans/PHASE_4_EXECUTION_PLAN.md`](PHASE_4_EXECUTION_PLAN.md)

---

## 2. Phase-by-Phase Milestone Tracker

### Phase 1: System Architecture, Design Specification & Mockups [COMPLETED]
- [x] **Requirements Engineering**: Functional (PTY lifecycle, agent SSE streaming, OSC 133 prompt detection, OSC 7 CWD tracking, dynamic tiling) and Non-Functional ($<8\text{ms}$ latency, $<45\text{MB}$ RAM, $>100\text{k lines/min}$).
- [x] **Architectural Alternatives Analysis**: Detailed comparison of Tauri 2.0 vs Electron vs Native Qt.
- [x] **Aesthetics & Interaction Design**: Kitty theme, spring-physics cursor shader formulation, Linux top-right window controls (`_ □ ✕`), direct edge/corner hover resizing.
- [x] **Vector SVG Mockups**:
  - [x] `docs/mockups/mockup_a_sidebar_expanded.svg` (2-col tiled layout with edge hover resizing and expanded sidebar).
  - [x] `docs/mockups/mockup_b_grid_tiles.svg` (3-col priority grid layout).
  - [x] `docs/mockups/mockup_c_stacked_zen.svg` (1-col wide stack layout).
- [x] **Edge Case & Protocol Hardening**: ANSI stripper for token efficiency, interactive prompt detection, `SIGWINCH` resize sync, shell crash recovery, global shortcuts.
- [x] **Repository Initialization & Plan Structuring**: Master plan tracker and phase execution plans.

---

### Phase 2: Tauri 2.0 & Rust Core PTY Engine [COMPLETED]
- [x] **Tauri 2.0 Project Scaffold**: Initialize `src-tauri` with Cargo dependencies (`portable-pty`, `tokio`, `serde`, `parking_lot`, `bytes`, `nix`).
- [x] **PTY Process Supervisor**: Implementation of `portable_pty::PtyPair` lifecycle, non-blocking asynchronous reader loops, process group signal dispatching (`SIGINT`, `SIGTERM`, `SIGKILL`).
- [x] **OSC 133 & OSC 7 Byte Parser**: Zero-allocation state machine for prompt boundaries, command exit code capture, and dynamic CWD tracking.
- [x] **Ring Buffer Storage**: 50,000-line circular buffer per terminal session for instant re-attachment and historical inspection.
- [x] **Automated Test Coverage**: 16 Rust unit and integration tests passing (`cargo test`).

---

### Phase 3: Embedded Local Agent API & Dual Streaming [COMPLETED]
- [x] **Axum Server Setup**: Localhost loopback binding (`127.0.0.1:7890`) with bearer token authentication guard.
- [x] **REST Endpoints**: `/api/v1/terminals` (create, list, inspect, resize, input, kill, close).
- [x] **SSE Dual-Streaming Protocol (`/exec`)**: Real-time stdout/stderr streaming, `stripAnsi` token optimization, interactive prompt detection (`event: prompt_waiting`), concurrency conflict guard (`409 Conflict`).
- [x] **WebSocket Protocol (`/ws`)**: Full-duplex bidirectional interactive raw streaming.
- [x] **Automated Test Coverage**: 10 integration tests in `api_tests.rs`, 15 unit tests in `lib.rs`, and 6 integration tests in `pty_tests.rs` (31 total passing tests).

---

### Phase 4: Desktop Canvas UI & Kitty Aesthetics [PLANNED / READY FOR EXECUTION]
- [ ] **Frontend Scaffold**: Vite + TypeScript + Kitty Tokyo Night design tokens.
- [ ] **Dynamic Tiling Grid**: Columns-per-row controller (`1`, `2`, `3`, `4` cols), top-insertion rule, infinite vertical scroll container, direct edge and corner drag-resizing with dynamic adjacent tile reflow.
- [ ] **Terminal Tile & Linux Header**: Linux top-right controls (`_ □ ✕`), dynamic status badges, copy-on-select clipboard integration, and in-tile buffer search (`Ctrl+Shift+F`).
- [ ] **Kitty Spring-Physics Cursor Shader**: WebGL/Canvas overlay tracking xterm cursor jumps with velocity-based decay.
- [ ] **Collapsible Sidebar Organizer**: Session tree, double-click to pin/move to top, instant remove buttons, and 3 sorting modes (Running Priority, MRU, Creation).
- [ ] **Dock Tray & Keybindings**: Minimized window dock tray and global shortcut manager (`Ctrl+Alt+N`, `Ctrl+Alt+1..4`, etc.).

---

### Phase 5: Verification, Benchmarking & Linux Packaging
- [ ] **Automated Unit & Integration Tests**: PTY lifecycle, OSC parser, SSE chunk stream integrity, tile resizing math.
- [ ] **High-Volume Stress Benchmark**: Stream $>100,000\text{ lines/min}$ verifying zero UI freezes ($>55\text{ FPS}$).
- [ ] **Platform Verification**: Wayland / X11 rendering stability, memory footprint verification ($<45\text{MB}$ idle).

---

### Phase 6: Model Context Protocol (MCP) Integration (Final Milestone)
- [ ] **MCP Server Adapter**: Embedded JSON-RPC stdio/SSE server (`termcmd --mcp`).
- [ ] **MCP Tool Schema Definitions**: `termcmd_create_terminal`, `termcmd_run_command`, `termcmd_send_input`, `termcmd_list_terminals`.
- [ ] **Agent Integration Testing**: End-to-end verification with MCP-compatible agent clients.
