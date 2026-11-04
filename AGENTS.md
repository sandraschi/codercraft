# CoderCraft — Sovereign Agentic IDE & Universal MCP Server

## Overview
CoderCraft is a sovereign agentic desktop development studio and MCP server built in native Rust + egui 0.37. It subsumes OpenCode, Cursor, and Zed into a single zero-panic desktop application with FastMCP JSON-RPC interoperability.

## Architecture & Crates
- `crates/codercraft-core`: File tree scanning, instruction discovery (`AGENTS.md`, `CLAUDE.md`, `.cursorrules`), linter abstraction (Clippy, Ruff, Biome), shell runner, and unified command dispatch bus.
- `crates/codercraft-agent`: Dual-mode agent loop (`Plan` for exploration and structured architecture roadmaps, `Build` for active code synthesis and refactoring proposals), wired to `craft-ai-core` for local (Ollama) and cloud (Claude, GPT, DeepSeek) token telemetry.
- `crates/codercraft-mcp`: MCP 2024-11-05 stdio server (`codercraft-cli --mcp`) and CLI client.
- `crates/codercraft-app`: Native immediate-mode egui desktop studio with central chat stream, collapsible thinking/plan blocks, diff inspector with green/red syntax highlighting, extensible MCP and terminal sidebars, and dark/light themes.

## Port & Protocol
- Control & UI Port: `14128` (assigned in `mcp-central-docs/operations/WEBAPP_PORTS.md`).
- MCP Transport: Stdio JSON-RPC 2.0 (MCP 2024-11-05 standard).

## Running
```powershell
# Headless smoke test
.\target\debug\codercraft.exe --smoke

# Run Desktop UI
.\target\debug\codercraft.exe

# Start MCP stdio server
.\target\debug\codercraft-cli.exe --mcp
```
