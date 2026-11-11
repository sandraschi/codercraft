# CoderCraft — Product Requirements Document (PRD)

**Document Identifier**: `CRAFT-PRD-2026-CODER-002`  
**Version**: 1.2.0  
**Date**: 2026-11-04  
**Author**: sandraschi fleet architecture / DeepMind AGY  
**Domain**: Local-First Agentic IDE, Stigmergic Code Generation, Multi-Monitor Studio  
**Standard Port**: 14128  

---

## 1. Problem Statement & High-Level Objective
Modern code intelligence tools suffer from severe fragmentation:
1. **Resource Hunger**: Electron-based IDEs consume gigabytes of memory and introduce input latency.
2. **Cloud Paywalls & Telemetry Leaks**: Proprietary AI editors upload codebases to third-party clouds and degrade context windows after minimal iteration.
3. **Multi-Monitor Neglect**: Modern workstations typically possess 2 to 4 displays, yet contemporary AI IDEs confine themselves to a single viewport.
4. **Isolated Fleet Ecosystems**: Individual applications (audio, CAD, robotics, databases) operate as detached silos rather than an integrated swarm.

**CoderCraft's Solution**:
CoderCraft is a native Rust, immediate-mode (egui 0.37 / wgpu) local-first development environment that subsumes OpenCode, Cursor, and Zed into a single zero-panic workstation.

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
- **AppCraft** (Port 14114): Local GUI & mobile framework.
- **RobotCraft** (Port 14115): URDF kinematics, Norirobotics A3, Saschibot & Noetix Bumi.
- **MoneyCraft** (Port 14116): Double-entry ledger & crypto portfolio.
- **DJCraft** (Port 14118): Universal DJ controller & stem separator.
- **ModCraft** (Port 14120): Audio synthesis & tracker workstation.
- **WriterCraft** (Port 14124): Distraction-free authoring suite.
- **CoderCraft** (Port 14128): Local-first agentic IDE & MCP server.
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

### 3.5 UI/UX Architecture & Menu Bar Standard
In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric Craft Workstation):** Persistent code/plan buffer viewport with surrounding toolbars and status HUD. Top quick button row acts as accelerators into the `View` menu. To add a new IDE tool, inspector, or display view:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar Toolbox / Outline drawer.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `File`: New File (`Ctrl+N`), Open Workspace / File (`Ctrl+O`), Open Recent ▸, Save (`Ctrl+S`), Save As... (`Ctrl+Shift+S`), Close Buffer (`Ctrl+W`), Quit (`Ctrl+Q`).
  - `Edit`: Undo (`Ctrl+Z`), Redo (`Ctrl+Y`), Cut (`Ctrl+X`), Copy (`Ctrl+C`), Paste (`Ctrl+V`), Select All (`Ctrl+A`), Find / Replace (`Ctrl+F`), Preferences (`Ctrl+,`).
  - `View`: Toggle File Tree, Toggle Terminal / Diagnostics, Toggle Swarm Supervisor, Toggle Fleet Depot, Zoom In/Out, Theme (`Dark`/`Light`).
  - `Insert`: Insert Code Snippet, Add Agent Task, Add Stigmergy Card, Generate Blueprint.
  - `Modify`: Format Buffer (`Shift+Alt+F`), Refactor Symbol, Apply AST Diff Patch, Toggle Plan/Build Mode.
  - `Commands`: Run Build, Run Tests, Run Clippy / Linter, Launch Stigmergic Swarm, Probe Fleet Depot.
  - `Window`: Quad-Display Layouts (Single, Dual, Quad), Reset Layout, Focus Next Display.
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+Shift+P` / `Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About CoderCraft`.
- **Behavioral Mechanics:** `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.


---

## 4. Verification & Safety Mandate
- **Zero-Panic Containment**: Every crate enforces `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`.
- **Smoke Testing**: Validated via `codercraft.exe --smoke` and `cargo test --workspace`.
- **Local Git Strictness**: Commits confined to local Goliath repo (no remote pushes).
