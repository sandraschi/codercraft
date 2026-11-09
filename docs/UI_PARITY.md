# CoderCraft UI Parity

**Stack:** `eframe` / `egui` 0.37 (L3 `codercraft-app`), calls L1/L2 dispatch
directly in-process over a background tokio runtime. No WebView2, no HTTP
sidecar for the desktop path.

## 1. Oracle loop

The studio window is judged against itself over time plus explicit layout
targets from `../PRD.md` section 3.1 (four displays: IDE, terminal, swarm
supervisor, fleet depot + diff inspector).

1. Launch with `--smoke` (headless CI proof the app boots).
2. Open the real window, screenshot each display role.
3. Compare against the previous golden + the PRD layout; fix the layout
   code, not the PNG.
4. Lock with `egui_kittest` golden snapshots (chat stream, diff inspector
   with green/red, depot cards with ONLINE/OFFLINE badges).

## 2. L3 requirements

- `--smoke` headless CI mode (exits 0, no window).
- Golden PNGs for: central chat + thinking blocks, 3-way diff inspector,
  swarm node graph cards, depot TCP status list.
- `glyph_source_scan` tofu guard on all checked-in UI strings.
- Multi-buffer editor: dirty-state marker, `Ctrl+S` save, AST symbol jump
  outline; rendering stays sub-millisecond, no Electron.
- L3 never spawns `codercraft-cli.exe` as a child; never routes GUI
  actions through JSON-RPC or localhost HTTP.

## 3. Current status

Shell and display roles are declared; goldens and kittest locks land with
editor milestone 1 (per README scope-cut). This file is the bar that
milestone must clear.

## 4. Screenshots

`docs/screenshots/` (to be added at milestone 1):

- `display-1-ide.png` - chat + buffers + symbol outline.
- `display-2-terminal.png` - shell + linter stream + test console.
- `display-3-swarm.png` - agent cards + task blackboard.
- `display-4-depot.png` - 12-app depot + diff inspector.
