# Legacy audit: ARSPaint v1 (Rust/egui)

ARSPaint v1 lives on the `legacy-rust` branch (tag `arspaint-v1-rust`). It is
preserved history, not a base for further work. This document records what
was there so good ideas aren't lost when the code isn't ported.

## What existed

A native Rust/egui painting app (`src/`) with:

- `image_store.rs` — `Vec<Layer>` + active index, selection mask (`GrayImage`),
  cached composite `RgbaImage`, file open/save, canvas resize.
- `layers.rs` — `Layer` with a `LayerData` enum (`Raster`, `Vector`, `Tone`),
  blend modes, vector shapes.
- `commands.rs` — cursor-based `CommandStack` for undo/redo, `PatchCommand`
  storing before/after rectangular pixel patches per commit.
- `tools/` — brush, eraser, line, rectangle, ellipse, curve, fill, eyedropper,
  rect/lasso selection, transform. Each tool owned a scratch `RgbaImage` the
  size of the canvas, painted to during drag, merged into the layer on release.
- `ui/` — a from-scratch immediate-mode shell: menu bar with branding, a
  grouped tools palette, a colors panel (HSV wheel + swatches), a layers
  panel, a history panel with click-to-jump, and a themed canvas widget with
  tested coordinate mapping.

Two unmerged branches (`engine-rewrite`, `paintdotnet-ui-shell`) show the
project actively converging on a Paint.NET-style layout and interaction
model — menu bar, left tool palette, right layers/colors/history docks,
undoable layer add/delete/move/duplicate/merge, per-layer alpha lock, a
trimmed Paint.NET-compatible shape set. This is exactly the direction v2
takes, just on a different foundation.

## What worked

- **Patch-based undo** (`PatchCommand`: layer index + rect + old/new sub-image)
  is a clean, cheap model for raster tools. Pinta's `HistoryItem`/
  `SimpleHistoryItem` system already does the equivalent (surface diffs), so
  no porting needed — but it validates the general approach.
- **Tool temp-layer pattern** (scratch buffer during drag, merged on release,
  shown as an overlay) is a sound interaction model. Pinta's tools already
  work this way via `Layer.Surface` snapshots + `ToolLayer`.
- **Single dirty flag driving re-composite** (`composite_dirty` →
  `update_textures()`) avoided per-frame recompositing. Worth keeping as a
  principle for any ARS-specific overlay rendering (reference view, split
  view) added on top of Pinta's canvas.
- The UI direction itself (Paint.NET-style chrome, dedicated docks, click-to-
  jump history) was the right call — it's carried forward, just onto Pinta's
  own docking/history system instead of reimplemented.

## What failed architecturally

- **The core mistake: building the editor from scratch.** Every subsystem a
  mature image editor needs — layer compositing, blend modes, selection
  tooling, history, file format I/O, add-ins, effects, color management,
  accessibility — was being reimplemented one piece at a time in `image`/
  `egui`. None of it is ARSPaint's actual value proposition; all of it is
  Pinta's existing, tested, maintained implementation.
- Vector layers and tone layers existed as `LayerData` variants but had no
  clear compositing/export story relative to raster layers — an example of
  scope growing sideways (a new layer *kind*) instead of forward.
- No test suite existed at all (per the old `CLAUDE.md`), on infrastructure
  code (compositing, undo, coordinate mapping) where regressions are exactly
  the kind of thing tests catch cheaply.
- egui's immediate-mode model makes a persistent, addressable widget tree
  (needed for docking, add-ins, accessibility semantics) something you fight
  the framework for rather than get for free, unlike GTK4/libadwaita.

## Ideas worth carrying forward into v2

- Semantic *tool* presets bound to single keys (the old tools panel already
  grouped tools for fast keyboard access) — maps directly onto ARSPaint v2's
  `ARS Pen` / `Pixel Pencil` presets and the `b`/`e`/`f`/`i`/`v` keyboard layer.
- Click-to-jump history panel — Pinta's history dock already supports this;
  no new work needed, just confirm it's wired up.
- The instinct to trim the shape/tool set down to a deliberate, curated list
  (v1's "trim shapes to Paint.NET geometric set" commit) rather than exposing
  everything — same instinct applies to which Pinta tools get ARS keyboard
  bindings first.
- Per-layer alpha lock and per-layer lock toggle (v1's last two commits) are
  useful UX; check whether Pinta's `UserLayer` already exposes lock semantics
  before re-adding.

## What is intentionally being abandoned

- The custom `RgbaImage`-based compositor, blend mode implementation, and
  patch-based `CommandStack` — replaced by Pinta's `Document`/`Layer`/
  `PaintSurface` and `DocumentHistory`.
- The custom tool trait hierarchy (`tools/base.rs` and implementations) —
  replaced by Pinta's `BaseTool` and the tools under `Pinta.Tools`.
- The from-scratch egui UI shell (`ui/*.rs`) — replaced by Pinta's GTK4/
  libadwaita `MainWindow`, docking system, and widget set.
- Vector and tone layer kinds — no equivalent is planned for v2 initially;
  revisit only if a concrete workflow need appears.
