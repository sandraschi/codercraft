# CoderCraft — System Architecture & Technical Specification

**Document Identifier**: `CRAFT-SPEC-2026-CODER-002`  
**Version**: 1.2.0  
**Date**: 2026-11-04  
**Author**: sandraschi fleet architecture / DeepMind AGY  
**Domain**: Rust Workspaces, egui 0.37 Immediate-Mode UI, FastMCP JSON-RPC, Multi-Display Viewports  
**Standard Port**: 14128  

---

## 1. Crate Hierarchy & Module Responsibilities

The `codercraft` repository is an ArtCraft 3-tier Rust workspace:

```
codercraft/
├── Cargo.toml
├── README.md
├── AGENTS.md
├── PRD.md
├── SPEC.md
├── start.bat
├── start.ps1
└── crates/
    ├── codercraft-core/
    │   ├── Cargo.toml
    │   ├── src/lib.rs            # Command bus, File ops, Linters, Fleet Depot
    │   └── tests/
    │       └── dispatch_tests.rs # 6 integration tests
    ├── codercraft-agent/
    │   ├── Cargo.toml
    │   └── src/lib.rs            # Dual-mode agent, Token metrics, SwarmLedger
    ├── codercraft-mcp/
    │   ├── Cargo.toml
    │   └── src/main.rs           # FastMCP stdio server & CLI
    └── codercraft-app/
        ├── Cargo.toml
        └── src/main.rs           # egui Desktop Studio & Quad-Display Viewports
```

---

## 2. Command Bus Protocol (`codercraft-core`)

All actions flow through the unified asynchronous dispatcher:
`pub async fn dispatch(cmd: Command, store: &DatabaseStore) -> serde_json::Value`

### Supported Commands
| Command | Parameters | Description |
|---|---|---|
| `Status` | None | Returns version, port, capabilities, active language |
| `OpenFile` | `path` | Validates file presence, buffer metadata |
| `ReadFileRange` | `path, start_line, end_line` | Reads line ranges with bounds checking |
| `WriteFile` | `path, content` | Atomic parent dir creation and file writing |
| `ApplyHunkEdit` | `path, target, replacement` | String hunk replacement |
| `GrepSearch` | `root, query` | Recursive text pattern matcher |
| `IndexWorkspace` | `workspace_dir` | Tree scanning + instruction discovery |
| `DiscoverInstructions` | `workspace_dir` | Discovers `AGENTS.md`, `CLAUDE.md`, `.cursorrules` |
| `SelectLanguage` | `language` | Configures language (Rust, Python, TS, Go, C++) |
| `RunLinter` | `workspace_dir, auto_fix` | Runs clippy, ruff, biome, golangci-lint, clang-tidy |
| `RunTestSuite` | `test_filter` | Executes `cargo test` in sandbox |
| `ConfigureMcpServer` | `entry` | Registers external MCP servers |
| `ListMcpServers` | None | Returns active MCP tool servers |
| `ExecuteConsoleCommand` | `command_line, working_dir` | Executes shell command with exit code capture |
| `ConfigureLlm` | `config` | Swaps Ollama, Claude, GPT, or DeepSeek models |
| `AgentRefactor` | `intent, target_file` | Synthesizes an AST unified patch |
| `AgentPlan` | `goal, workspace_dir` | Formulates a phased implementation plan |
| `ExtractSymbols` | `path` | Extracts top-level fn, struct, enum symbols |
| `ListFleetProjects` | None | Returns all 12 Craft fleet apps with ports |
| `ProbeFleetProjects` | None | Actively probes TCP ports and measures ping latencies |
| `LaunchFleetProject` | `folder_name` | Executes `start.bat` for the designated app |
| `GetSwarmState` | None | Returns workers, tasks, pheromones, token spend |
| `StepSwarm` | None | Advances task state machine and decays pheromones |

---

## 3. Stigmergic Swarm Supervisor Subsystem (`codercraft-agent`)

The swarm models environmental stigmergy without requiring centralized lockstep coordination:
- **Workers**:
  - `Architect-01`: Scans architecture and generates phased implementation plans.
  - `Synthesizer-02`: Writes atomic code diffs satisfying user intent.
  - `ClippyGuard-03`: Runs static analysis and verifies zero-panic containment.
  - `TestRunner-04`: Executes automated test suites and asserts correctness.
  - `DocSync-05`: Synchronizes `PRD.md`, `SPEC.md`, and `AGENTS.md`.
- **Environmental Task Blackboard**:
  - State transitions: `Proposed` $\to$ `Claimed` $\to$ `Synthesized` $\to$ `Verified` $\to$ `Committed`.
  - Pheromone signal: Environmental intensity $\tau \in [0.0, 1.0]$. Decays at rate $\lambda = 0.98$ per simulation tick. Error signals trigger immediate priority repair.

---

## 4. Multi-Display Quad Layout Implementation (`codercraft-app`)

Built with `egui 0.37` and `eframe`:
- `StudioScreenMode`:
  - `IdeStudio`: Primary workstation (Display 1).
  - `FleetDepot`: One-click project launcher (Display 4).
  - `SwarmSupervisor`: Stigmergic swarm live blackboard (Display 3).
  - `QuadDisplayConfig`: Multi-monitor layout manager.
- Native viewports can be detached into independent native OS windows spanning 4 physical monitors.

---

## 5. FastMCP 2024-11-05 Tool Registry (`codercraft-mcp`)

Stdio JSON-RPC tools exposed to Claude, Antigravity, and fleet subagents:
- `codercraft_status`
- `codercraft_open_file`
- `codercraft_read_file`
- `codercraft_write_file`
- `codercraft_apply_edit`
- `codercraft_grep`
- `codercraft_index_workspace`
- `codercraft_plan`
- `codercraft_refactor`
- `codercraft_lint`
- `codercraft_shell`
- `codercraft_fleet_depot`
- `codercraft_fleet_launch`
- `codercraft_swarm_status`
