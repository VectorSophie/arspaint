# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # debug build
cargo run            # run the app
cargo build --release  # optimized release build
cargo check          # fast type-check without linking
cargo clippy         # lint
```

There is no test suite at this time.

## Architecture

ArsPaint is a native desktop painting application built with **eframe/egui** (immediate-mode GUI). The pixel data lives in the `image` crate's `RgbaImage` buffers; the GPU only ever sees textures uploaded via `egui`.

### Data flow

```
Tool (stroke/shape input)
  → temp RgbaImage layer (shown via layer_texture during drag)
  → on release: commits pixels to ImageStore, returns PatchCommand
      → CommandStack stores PatchCommand for undo/redo
          → ImageStore::mark_dirty() → composite() → base_texture upload
```

### Core modules

| Module | Responsibility |
|---|---|
| `state.rs` | `AppState` — owns `ImageStore`, `CommandStack`, `active_tool`, `ToolSettings`, `Keybindings`, palette |
| `image_store.rs` | `ImageStore` — owns the `Vec<Layer>`, active layer index, selection mask, and cached composite `RgbaImage`; handles file open/save and canvas resize |
| `layers.rs` | `Layer` struct with `LayerData` enum (`Raster`, `Vector`, `Tone`), `BlendMode`, `VectorShape` |
| `commands.rs` | `Command` trait + `CommandStack` (cursor-based undo/redo); `PatchCommand` stores old/new rectangular pixel patches |
| `tools/base.rs` | `Tool` trait; `BrushTool`, `EraserTool`, `LineTool` implementations |
| `tools/rect.rs` / `ellipse.rs` | `RectangleTool`, `EllipseTool` |
| `tools/selection.rs` | `RectSelectionTool`, `LassoSelectionTool` — write to `ImageStore::selection` (a `GrayImage` mask) |
| `tools/transform.rs` | `TransformTool` |
| `ui.rs` | `ArsApp` (implements `eframe::App`); owns zoom/pan state and three egui texture handles; renders top toolbar, right layers panel, and central canvas |

### Key design invariants

**Tool temp layer pattern**: Every drawing tool maintains its own `RgbaImage` buffer the same size as the canvas. While dragging, strokes are written to this temp buffer (shown as `layer_texture` overlaid on the composite). On `is_released`, the tool merges the temp buffer into the active layer's raster buffer, clears the temp buffer, and returns a `PatchCommand` capturing the before/after patch for undo.

**Compositing**: `ImageStore::composite_dirty` is the sole dirty flag. Tools call `image.mark_dirty()` after commits. `ArsApp::update_textures()` re-uploads to GPU only when `self.image_dirty` is set (set when a `PatchCommand` is returned or undo/redo runs). The `composite()` method iterates all visible layers bottom-to-top, applying opacity, blend modes (Normal/Multiply/Add/Screen), and clipping masks using `blend_buffer_static`.

**Selection mask**: Stored as `ImageStore::selection: Option<GrayImage>`. Tools check it pixel-by-pixel at commit time — pixels where the mask value is 0 are skipped. Deselect simply sets `selection = None`.

**Alpha lock**: Per-layer `alpha_locked` flag. At commit time in each tool, if alpha lock is on, pixels are only written where the existing target pixel already has `alpha > 0`, and the written pixel's alpha is replaced with the target's alpha.

**Undo/redo**: `CommandStack` uses a cursor index into a `Vec<Box<dyn Command>>`. Pushing a new command truncates any undone future. `PatchCommand` stores `(layer_index, x, y, old_patch, new_patch)` as cloned sub-images.

**Adding a new tool**: Implement the `Tool` trait from `tools/base.rs` (five methods: `name`, `update`, `get_temp_layer`, `draw_cursor`, `configure`), add a `pub use` in `tools/mod.rs`, and wire up a button + keybinding in `ui.rs`.
