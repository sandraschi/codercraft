# CoderCraft — Product Requirements Document (PRD)

**Document Identifier**: `CRAFT-PRD-2026-CODER-002`  
**Version**: 1.2.0  
**Date**: 2026-11-04  
**Author**: sandraschi fleet architecture / DeepMind AGY  
**Domain**: Sovereign Agentic IDE, Stigmergic Code Generation, Multi-Monitor Studio  
**Standard Port**: 14128  

---

## 1. Problem Statement & High-Level Objective
Modern code intelligence tools suffer from severe fragmentation:
1. **Resource Hunger**: Electron-based IDEs consume gigabytes of memory and introduce input latency.
2. **Cloud Paywalls & Telemetry Leaks**: Proprietary AI editors upload codebases to third-party clouds and degrade context windows after minimal iteration.
3. **Multi-Monitor Neglect**: Modern workstations typically possess 2 to 4 displays, yet contemporary AI IDEs confine themselves to a single viewport.
4. **Isolated Fleet Ecosystems**: Individual applications (audio, CAD, robotics, databases) operate as detached silos rather than an integrated swarm.

**CoderCraft's Solution**:
CoderCraft is a native Rust, immediate-mode (egui 0.37 / wgpu) sovereign development environment that subsumes OpenCode, Cursor, and Zed into a single zero-panic workstation.

---

## 2. Target Persona & User Modes
- **Dual Cognitive Modalities**:
  - **Plan Mode (`@plan`)**: Non-destructive, read-only architectural exploration. Synthesizes structured implementation roadmaps, parses workspace rules (`AGENTS.md`, `CLAUDE.md`, `.cursorrules`), and formulates phased execution checklists without modifying files.
  - **Build Mode (`@build`)**: Active modification mode. Employs autonomous AST diff patching, runs continuous background test/linter feedback loops, and commits changes upon full verification.
- **Audience Range**:
  - Beginners ("Little Timmy Mode"): High-level conversational intent with immediate visual feedback.
  - Systems Engineers ("Linus Mode"): Sub-millisecond text rendering, low-level multi-terminal console, AST inspection, zero Electron overhead.

---

## 3. Key Feature Specifications

### 3.1 Quad-Display Multi-Monitor Architecture
CoderCraft detects and provisions workstations across up to 4 physical or virtual displays:
- **Display 1 (Primary IDE Workstation)**:
  - Central Agentic Chat & Code Buffer.
  - Workspace File Tree & Open Buffers.
  - Language selector, FastMCP registry, and Model config.
- **Display 2 (Terminal & Build Swarm)**:
  - Multi-tab interactive PowerShell / Bash sandbox.
  - Continuous Clippy, Ruff, and Biome diagnostic streaming.
  - Background test runner console with exit code piping.
- **Display 3 (Stigmergic Swarm Supervisor)**:
  - Live Specialist Agent Worker Cards (`Architect-01`, `Synthesizer-02`, `ClippyGuard-03`, `TestRunner-04`, `DocSync-05`).
  - Environmental Task Blackboard (`Proposed`, `Claimed`, `Synthesized`, `Verified`, `Committed`).
  - Pheromone signal intensity indicators and task decay simulation.
  - Global token budget accounting and compiler pass rate telemetry.
- **Display 4 (Fleet Depot & Diff Reviewer)**:
  - One-click launch depot for all 12 Craft fleet applications.
  - Split-screen 3-way AST Diff Inspector with green additions (`+`) and red deletions (`-`).
  - Webview and port monitors (14111..14131).

### 3.2 Fleet Craft Project Depot
Direct integration into all fleet applications with standardized port mappings:
- **DatabaseCraft** (Port 14113): Embedded SQLite + Vector RAG storage.
- **AppCraft** (Port 14114): Sovereign GUI & mobile framework.
- **RobotCraft** (Port 14115): URDF kinematics, Norirobotics A3, Saschibot & Noetix Bumi.
- **MoneyCraft** (Port 14116): Double-entry ledger & crypto portfolio.
- **DJCraft** (Port 14118): Universal DJ controller & stem separator.
- **ModCraft** (Port 14120): Audio synthesis & tracker workstation.
- **WriterCraft** (Port 14124): Distraction-free authoring suite.
- **CoderCraft** (Port 14128): Sovereign agentic IDE & MCP server.
- **TravelCraft** (Port 14129): Offline GIS & Baedeker navigator.
- **MakerCraft** (Port 14130): Parametric CAD/CAM & 3D slicer.
- **SportsCraft** (Port 14131): Biometric kinetic coach.
- **DemoCraft** (Port 14111): Showcase & A2 print poster hub.

Each project card provides one-click `[🚀 Start App]`, `[📂 Open in IDE]`, and `[🌐 Webapp]`.

### 3.3 FastMCP 2024-11-05 Server & Client
- Headless stdio server (`codercraft-cli --mcp`) communicating over JSON-RPC 2.0.
- Standard tools for file read/write/edit, grep searching, workspace indexing, architectural planning, code refactoring, AST symbol extraction, TCP socket fleet depot probing, and swarm monitoring.

### 3.4 Visual Aesthetics & Cognitive Ergonomics
- **Tokyo Night / Cyberpunk Slate Palette**: Sleek dark theme with deep panel backgrounds, cyan accent highlights, emerald completion badges, and purple token telemetry.
- **Bézier Node Graph Spaghetti Canvas**: ComfyUI / MaterialCraft style node canvas connecting worker agents to environmental tasks with cubic Bézier curves and animated pulse dots.
- **Multi-Buffer Code Studio**: Tabbed source file editing, AST symbol outline drawer, character/line telemetry, and one-click disk persistence.


---

## 4. Verification & Safety Mandate
- **Zero-Panic Containment**: Every crate enforces `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`.
- **Smoke Testing**: Validated via `codercraft.exe --smoke` and `cargo test --workspace`.
- **Local Git Strictness**: Commits confined to local Goliath repo (no remote pushes).
