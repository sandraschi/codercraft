# CoderCraft

**CoderCraft** is an ArtCraft-tier sovereign agentic integrated development environment (IDE) that subsumes the capabilities of OpenCode, Cursor, Zed, and Antigravity into a unified, zero-runtime-tax native binary.

Designed from first principles with a dual cognitive UX:
- **Timmy Mode (Age 8)**: Visual block-and-canvas logic flow, plain-English voice/text prompts, gamified diffs, and instant visual run/preview.
- **Linus Torvalds Mode (Kernel/Hacker)**: Raw tree-sitter AST queries, sub-millisecond wgpu/GPU text renderer, stigmergic multi-agent patch proposals with automated compiler gatekeeper verification, zero-panic Rust safety containment.

---

## Architecture (ArtCraft 3-Tier Parity)

1. `crates/codercraft-core`: Language server client, AST tree-sitter parser, unified buffer management, and compiler gatekeeper.
2. `crates/codercraft-agent`: Stigmergic agentic swarm engine for autonomous multi-file refactoring, patch generation, and test synthesis.
3. `crates/codercraft-mcp`: FastMCP JSON-RPC server on port `14128` exposing `open_buffer`, `propose_refactor`, `run_ast_query`, and `agent_swarm_status`.
4. `crates/codercraft-app`: Native GUI (`eframe`/`egui`) with dual Timmy/Linus toggle, split-diff editor, and CLI binary (`codercraft-cli`).

---

## Verification & Usage

```powershell
# Build entire workspace
cargo build

# Run unit and dispatch tests
cargo test

# Headless smoke test
./target/debug/codercraft.exe --smoke

# Command Line Interface
./target/debug/codercraft-cli.exe open ./Cargo.toml
./target/debug/codercraft-cli.exe refactor "replace old helper with vectorized SIMD"
```
