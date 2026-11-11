# CoderCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(codercraft-core)       (codercraft-mcp)       (codercraft-app)       (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Domain Engine & Bus (`codercraft-core`, `codercraft-agent`)
- [x] L1 `Command::dispatch` universal execution bus.
- [x] Workspace rule indexing (`AGENTS.md`, `CLAUDE.md`, `.cursorrules`).
- [x] Deterministic AST diff and patch engine.
- [x] Plan Mode (`@plan`) vs Build Mode (`@build`) modalities.
- [x] Stigmergic supervisor swarm blackboard and pheromone state.
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`codercraft-mcp`)
- [x] Single CLI binary `codercraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0.
- [x] Complete tool catalog mirroring `Command::dispatch` operations.
- [x] Background test/linter runners (Clippy, Ruff, Biome) streaming diagnostics.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`codercraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` quad-display workstation.
- [x] Multi-buffer code editor with syntax highlighting and AST symbol drawer.
- [x] Stigmergic swarm supervisor canvas with Bézier task links.
- [x] Fleet depot panel with live TCP reachability probes (ports 14111..14131).

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [ ] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [ ] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About CoderCraft`).
- [ ] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [ ] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [ ] Headless `--smoke` parity validation.
- [ ] Standalone release packaging and current-user NSIS installer.
