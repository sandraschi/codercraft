# CoderCraft — Comprehensive Help, Architecture & Competitive Matrix

**Document Identifier**: `CRAFT-HELP-2026-CODER-003`  
**Application**: CoderCraft (Sovereign Agentic IDE & Studio)  
**Version**: 0.2.0 | **Port**: 14128  

---

## 1. What CoderCraft Is and Does

**CoderCraft** is a pure-Rust, zero-runtime-tax, immediate-mode development environment and agentic coding studio. It is designed from first principles to subsume the capabilities of **Claude Desktop, OpenCode, Cursor, and Antigravity (AGY)** into a single sovereign desktop application.

### Core Capabilities:
1. **Zero Electron Runtime Overhead**: Built using `egui 0.37` and `wgpu`. Starts in under 30 milliseconds, consumes ~35 MB of RAM (versus 1.5–3.0 GB for Electron IDEs), and renders at a silky 60–144 FPS.
2. **Dual-Mode Cognitive Modalities (OpenCode Parity)**:
   - **Plan Mode (`📋 Plan`)**: Non-destructive exploration. Discovers workspace rules (`AGENTS.md`, `CLAUDE.md`, `.cursorrules`), searches symbol declarations, and constructs structured phased implementation plans without modifying a single file.
   - **Build Mode (`🔨 Build`)**: Active code synthesis. Generates atomic AST code patches, validates diffs, applies replacements, and verifies changes against compiler and test loops.
3. **Multi-Buffer Code Studio & AST Symbol Outline**:
   - Tabbed editing of multiple source files with dirty-state tracking (`*`) and instant disk persistence (`Ctrl+S`).
   - Built-in AST outline parser automatically extracts top-level functions (`fn`, `def`), structs (`struct`, `class`), enums, and traits with exact line numbers for rapid jump navigation.
4. **Stigmergic Swarm Supervisor (Display 3)**:
   - Deploys a team of 5 specialized agents (`Architect-01`, `Synthesizer-02`, `ClippyGuard-03`, `TestRunner-04`, `DocSync-05`).
   - Interactive ComfyUI-style Bézier node graph with animated pheromone flow particles connecting workers to environmental tasks.
   - Self-healing feedback loops: compiler or linter errors drop priority repair pheromones that guide agents to fix regressions before code is committed.
5. **Multi-Display Quad Layout Architecture**:
   - Provisions up to 4 native displays:
     - **Display 1 (Primary)**: Code Editor & Central Agentic Chat.
     - **Display 2 (Terminal)**: Interactive PowerShell/Bash sandbox & streaming linter.
     - **Display 3 (Supervisor)**: Stigmergic swarm node graph & task blackboard.
     - **Display 4 (Fleet Depot)**: One-click project launcher & split diff inspector.
6. **Fleet Craft Project Depot**:
   - Embedded mission control for all 12 sovereign Craft applications (DatabaseCraft, AppCraft, RobotCraft, MoneyCraft, DJCraft, ModCraft, WriterCraft, CoderCraft, TravelCraft, MakerCraft, SportsCraft, DemoCraft).
   - Live non-blocking TCP socket probing measures exact network round-trip ping latencies (`● ONLINE (1ms)` / `○ OFFLINE`).
7. **FastMCP 2024-11-05 Dual Server & Client**:
   - 100% JSON-RPC 2.0 stdio server (`codercraft-cli --mcp`) exposing tools for file I/O, regex grep, AST symbols, planning, patching, fleet probing, and swarm monitoring.
8. **Zero-Panic Safety Guarantee**:
   - Enforces `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, and `#![deny(clippy::panic)]` across all workspace crates.

---

## 2. In-Depth Comparative Matrix: CoderCraft vs Claude Desktop vs OpenCode vs Antigravity (AGY)

| Dimension | Claude Desktop | OpenCode | Antigravity (AGY) | CoderCraft |
|---|---|---|---|---|
| **Runtime & Tech Stack** | Electron / Node.js (~800MB–2GB RAM) | Node.js / TypeScript + Electron / Ink (~600MB–1.5GB) | Python / Node CLI harness + Webview | **100% Native Rust (`egui` + `wgpu`) (~35MB RAM, 144 FPS)** |
| **Startup Latency** | 2.5 – 5.0 seconds | 1.8 – 4.0 seconds | 1.0 – 2.5 seconds | **< 30 milliseconds** |
| **Multi-Display Support** | Single window | Single window / terminal | Single workspace window | **Native Quad-Display Viewports (Displays 1, 2, 3, 4)** |
| **Agent Workflow Modes** | Single-turn / conversational | Dual `Plan` vs `Build` modes | Autonomous pair-programming with slash commands | **Subsumes OpenCode Dual Modes + Stigmergic Swarm + AGY pair programming** |
| **Swarm Visualization** | None | Text-based subagent logs | Subagent tree in chat | **Interactive Bézier Spaghetti Node Graph (ComfyUI / MaterialCraft style)** |
| **Multi-Buffer Editor** | Read-only artifacts | Integrated file diffs | Integrated Monaco editor | **Native Multi-Buffer Editor with AST Symbol Jump Outline** |
| **Fleet / Multi-Project Mission Control** | None | Single repo context | Cross-directory navigation | **Built-in Fleet Depot with live TCP socket probing for 12 Craft apps** |
| **MCP Protocol Support** | Client only | Client & Server (TypeScript) | Client & Server (Python/Node) | **FastMCP 2024-11-05 JSON-RPC Server & Client in native Rust** |
| **Data Sovereignty & Offline AI** | Cloud-only (Anthropic API) | Cloud + Ollama | Cloud-driven | **100% Sovereign Offline-First (`craft-ai-core` Ollama + Cloud optional)** |
| **Compiler & Lint Containment** | None | Shell command execution | Harness verification | **Self-healing stigmergic loop + Zero-Panic compiler denies** |

---

## 3. Special Features & "Superpowers"

### 1. The Stigmergic Pheromone Loop
Unlike standard chatbots where errors must be manually copy-pasted back and forth, CoderCraft's `ClippyGuard-03` and `TestRunner-04` workers continuously deposit environmental error signals into the task blackboard upon any regression. `Synthesizer-02` senses the decaying pheromone gradient and synthesizes immediate restorative AST patches without wasting conversational context.

### 2. Live TCP Socket Probe & Fleet Launch
Every Craft app's standard port (14111..14131) is monitored via asynchronous TCP connect handshakes. You can launch, stop, inspect, or map any Craft app's buffers directly into CoderCraft with a single click.

### 3. Native Quad Viewports
When multiple monitors are connected, CoderCraft can undock Displays 2, 3, and 4 into independent OS windows, letting you dedicate an entire physical display to your terminal swarm and another to the live swarm node graph.

---

## 4. Keybindings & Command Shortcuts

- `Ctrl + S`: Save currently active buffer to disk.
- `F1` or `Help Menu`: Open this comprehensive manual and comparison matrix.
- `Screens -> Multi-Display`: Toggle quad-screen viewports across physical monitors.
- `Sub-toolbar [🔨 BUILD] / [📋 PLAN]`: Switch cognitive mode with one click.
- `Sub-toolbar [🚀 Fleet Depot]`: Access all 12 Craft apps with real-time port telemetry.
- `Sub-toolbar [🐝 Swarm Supervisor]`: View the live Bézier node graph.
