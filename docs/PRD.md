# CoderCraft PRD (docs pointer)

**Canonical spec:** [`../PRD.md`](../PRD.md) (`CRAFT-PRD-2026-CODER-002`, v1.2.0)
**Companions:** [`../SPEC.md`](../SPEC.md), [`../HELP.md`](../HELP.md), [`../AGENTS.md`](../AGENTS.md)
**Control port:** 14128 (`codercraft --control 14128`)
**Crates:** `codercraft-core` (L1 tree/lint/shell/bus) / `codercraft-agent`
(Plan + Build loops) / `codercraft-mcp` (L2 `codercraft-cli --mcp`) /
`codercraft-app` (L3 egui quad-display studio)

## Summary (do not drift; edit ../PRD.md instead)

CoderCraft is the local-first agentic IDE: Plan mode (read-only codebase
exploration, `AGENTS.md`/`CLAUDE.md`/`.cursorrules` discovery, phased
roadmaps) plus Build mode (AST diffs, atomic patches, test/linter gates),
with a stigmergic 5-agent supervisor swarm and a fleet depot (12 craft apps,
live TCP probe) in one native Rust binary (~35 MB RAM, <30 ms startup).

## Tier mapping
See [`TIERED_PLAN.md`](TIERED_PLAN.md) for full execution breakdown:
- Tier 1: workspace + deny gates + `Command::dispatch` + deterministic edit concept + L3 parity test + release size note.
- Tier 2: live linter/test runners (Clippy, Ruff, Biome) streaming in the terminal display; `craft-ai-core` token telemetry (Ollama local, cloud optional).
- Tier 3: desktop quad-display studio (`codercraft-app`), multi-buffer editor, agent swarm blackboard.
- Tier 4: Craft Menu Bar Standard (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`, mandatory 5-dialog Help suite, View toggles, Tofu Guard glyph scans, `egui_kittest` goldens, current-user NSIS installer, cold-boot table in README.

## Non-goals (from README)

Not an opencode wrapper; no "universal server" claims before proof; editor
milestone before agent milestone per the scope-cut note.

## Gates

`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo test --workspace`, `cargo fmt --all --check`, byte-exact MCP stdio
probe, `Cargo.lock` committed, no `target/` / `*.bak` / `*.exe` in status.
