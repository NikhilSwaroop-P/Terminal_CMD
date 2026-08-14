# TermCMD Master Plan & Milestone Tracker

---

## 1. Project Overview & Source of Truth

**TermCMD** is a specialized Linux desktop terminal canvas and orchestration manager built on Tauri 2.0 (Rust + WebGL TypeScript). It enables AI agents to execute, stream, and persist pseudo-terminal (PTY) sessions via local REST/SSE/WS APIs while providing human operators with a GPU-accelerated, Kitty-styled multi-window desktop canvas.

- **Primary Source of Truth (Design Spec)**: [`docs/DESIGN_DOC.md`](../docs/DESIGN_DOC.md)
- **Visual Mockups**: [`docs/mockups/`](../docs/mockups/)
- **Phase 1 Execution Plan**: [`plans/PHASE_1_EXECUTION_PLAN.md`](PHASE_1_EXECUTION_PLAN.md)
- **Phase 2 Execution Plan**: [`plans/PHASE_2_EXECUTION_PLAN.md`](PHASE_2_EXECUTION_PLAN.md)
- **Phase 3 Execution Plan**: [`plans/PHASE_3_EXECUTION_PLAN.md`](PHASE_3_EXECUTION_PLAN.md)

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

### Phase 3: Embedded Local Agent API & Dual Streaming [PLANNED / READY FOR EXECUTION]
- [ ] **Axum Server Setup**: Localhost loopback binding (`127.0.0.1:7890`) with bearer token authentication guard.
- [ ] **REST Endpoints**: `/terminals` (create, list, inspect, resize, input, kill, close).
- [ ] **SSE Dual-Streaming Protocol (`/exec`)**: Real-time stdout/stderr streaming, `stripAnsi` token optimization, interactive prompt detection (`event: prompt_waiting`), concurrency conflict guard (`409 Conflict`).
- [ ] **WebSocket Protocol (`/ws`)**: Full-duplex bidirectional interactive raw streaming.
- [ ] **Automated Test Coverage**: Unit and integration tests for API endpoints, SSE streams, authentication, and ANSI filtering.

---

### Phase 4: Desktop Canvas UI & Kitty Aesthetics
- [ ] **Frontend Scaffold**: Vite + TypeScript + Vanilla CSS design tokens.
- [ ] **Dynamic Tiling Grid**:
  - Columns-per-row controller (`1`, `2`, `3`, `4` cols).
  - Top-insertion rule for newly spawned terminals.
  - Infinite vertical scrolling container.
  - Direct edge and corner drag-resizing with dynamic adjacent tile reflow.
- [ ] **Collapsible Sidebar Organizer**:
  - Session tree with running/idle/error status pills.
  - Double-click to pin/scroll to top.
  - Instant remove/kill button (`✕`).
  - Sorting modes: Running Priority, Recent Activity (MRU), Creation Order.
- [ ] **Terminal Canvas & Kitty Cursor FX**:
  - WebGL-accelerated `xterm.js` rendering.
  - Spring-damper physics cursor trail shader overlay ($m\mathbf{p}'' + c\mathbf{p}' + k\Delta\mathbf{p} = 0$).
  - Linux top-right window controls (`_ □ ✕`).
  - Minimized dock tray and copy-on-select integration.

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
