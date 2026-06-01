# ArsPaint Phase A — Modern MS Paint Remake

**Date:** 2026-06-02  
**Status:** Approved  
**Scope:** Phase A only. Phase D (digital painting) builds on top of this.

---

## Goal

Refactor ArsPaint into a faithful Rust-native remake of modern Windows 10/11 Paint — same tools, same ribbon layout, same canvas behaviour — running on a TokyoNight dark theme. Phase A ends when an avid Paint user can open the app and do everything they'd do in Paint without reaching for Paint itself.

Reference: [JS Paint](https://github.com/1j01/jspaint) (MIT) for exact tool behaviour; [ReactOS mspaint](https://github.com/reactos/reactos/tree/master/base/applications/mspaint) for C++ implementation details.

---

## What Stays Unchanged

| File | Reason |
|---|---|
| `commands.rs` | PatchCommand + CommandStack are correct and sufficient |
| `layers.rs` | Keep file; Vector/Tone variants just won't be reachable from UI in Phase A |
| `image_store.rs` | Keep Vec<Layer> internals; Phase A always uses layer 0, composite() is trivial |
| `Tool` trait contract | All new tools implement the same 5-method interface |
| TokyoNight visuals | All `egui::Visuals` overrides in `ArsApp::new()` are untouched |

---

## Architecture Changes

### `state.rs`

- `ToolSettings`: add fields for `airbrush_radius: f32`, `text_font_size: f32`, `text_bold: bool`, `text_italic: bool`, `shape_fill_mode: FillMode` (enum: Outline / Fill / Both), `stroke_size: StrokeSize` (enum: Thin/Medium/Thick/ExtraThick → 1/3/5/8 px)
- `AppState`: add `color1: Rgba<u8>`, `color2: Rgba<u8>` (rename from `primary_color`/`secondary_color`), `clipboard: Option<RgbaImage>`, `floating_selection: Option<FloatingSelection>`
- `Keybindings`: update to modern Paint defaults (see Keybindings section)
- Remove `palette: Vec<Rgba<u8>>` from AppState — palette is now a fixed constant in the ribbon

### `ui.rs`

Complete rebuild of layout. `ArsApp` gains:
- `ribbon_tab: RibbonTab` (enum: Home / View)
- `show_rulers: bool`
- `show_grid: bool`  
- `show_status_bar: bool`

Panel structure (top → bottom):
1. `egui::TopBottomPanel::top("ribbon")` — renders ribbon (tab bar + tab content)
2. `egui::CentralPanel` — canvas
3. `egui::TopBottomPanel::bottom("status_bar")` — status bar (togglable)

Remove: `render_layers_panel()`, the right side panel, `show_shortcuts` popup, `remapping` field.

### `tools/base.rs`

- `EraserTool`: replace hardcoded `Rgba([255,255,255,255])` with `color` parameter (Color 2 passed in from AppState)
- `BrushTool`: rename to represent the brush tool; pencil is a separate 1px tool

---

## Ribbon UI

### Tab Bar

Two tabs: **Home** | **View**. Active tab highlighted with `#7aa2f7` underline on `#16161e` background.

### Home Tab Groups

**Clipboard**
- Paste (Ctrl+V) — large button
- Cut (Ctrl+X), Copy (Ctrl+C) — smaller row below

**Image**
- Select ▾ (dropdown: Rectangular Selection, Free-form Selection, Select All Ctrl+A, Invert Selection)
- Crop (crops canvas to current floating selection bounding box)
- Resize (opens Resize/Skew dialog — see Image Operations)
- Rotate ▾ (dropdown: Rotate 90° right, Rotate 90° left, Rotate 180°, Flip horizontal, Flip vertical)

**Tools** — 3-column icon grid + stroke size picker
- Row 1: Pencil (P), Fill (F), Color Picker (I)
- Row 2: Text (T), Eraser (E), Magnifier (M / Ctrl+scroll)
- Stroke size: 4 preset lines (1px / 3px / 5px / 8px), shown as horizontal bars of increasing thickness. Applies to Pencil, Brush, Eraser, Line, Curve, shape outlines.

Note: **Brush (B)** and **Airbrush (A)** are not in modern Paint's ribbon but are retained as keyboard-only tools to preserve the existing implementation and ease the Phase D transition. They do not appear in the Tools grid.

**Shapes** — gallery + fill mode
- 23-shape gallery in 2 rows of scrollable buttons (see Full Shape List)
- Below gallery: Outline / Fill / Both selector (3 radio options, shown when any shape tool is active)

**Colors**
- Color 1 swatch (left-click to set via color picker dialog)
- Color 2 swatch (left-click to set via color picker dialog)
- 20-color palette grid (left-click → set Color 1, right-click → set Color 2)
- "Edit colors" button → opens `egui::Window` with `egui::color_picker::color_picker_color32`

### View Tab Groups

**Zoom**
- Zoom In (+), Zoom Out (−)
- 100% button (reset zoom)
- Fit button (fit canvas to window)

**Show or hide**
- Rulers checkbox (horizontal + vertical pixel rulers along canvas edges)
- Gridlines checkbox (pixel grid, visible at ≥ 400% zoom)
- Status bar checkbox

---

## Full Tool List

### Drawing Tools

| Tool | Key | Behaviour |
|---|---|---|
| Pencil | P | 1px freehand. Left = Color 1, Right = Color 2. No anti-aliasing. |
| Brush | B | Circle stamp freehand, configurable size via stroke preset. Stabilization stays from existing impl. |
| Airbrush | A | Random dots within radius around cursor while held. Density ~30 dots/frame. |
| Eraser | E | Paints Color 2 (not hardcoded white). Size from stroke preset. |
| Fill | F | Flood fill from click point. 4-connected BFS. Left = Color 1, Right = Color 2. Respects selection mask if active. |
| Color Picker | I | Left-click → set Color 1. Right-click → set Color 2. Samples composite pixel. |
| Text | T | Click canvas → floating text input box appears. Font family, size, Bold, Italic in a small toolbar that appears in ribbon Tools area. Background: Opaque (Color 2) or Transparent. Committed to canvas on click-away or Enter. Not editable after commit. |
| Magnifier | M | Click to zoom in 2×. Right-click to zoom out. Ctrl+scroll also zooms. |

### Line Tools

| Tool | Behaviour |
|---|---|
| Line | Click+drag. Hold Shift → constrain to 0°/45°/90°. Stroke size from preset. Left = Color 1, Right = Color 2. |
| Curve | Click start → click end → drag to bend (one control point, quadratic Bézier). Second drag adds second control point (cubic). Double-click or release to commit. |

### Shape Tools (all 23)

All shape tools: left-click+drag draws with Color 1, right-click+drag draws with Color 2. Hold Shift → constrain to equal width/height (square, circle, etc.). Fill mode (Outline/Fill/Both) from ribbon selector.

Polygon tool: click to add vertices, double-click to close and commit.

**Basic geometry:** Rectangle, Rounded Rectangle, Oval, Triangle, Right Triangle, Diamond, Pentagon, Hexagon

**Arrows (7):** Right Arrow, Left Arrow, Up Arrow, Down Arrow, 4-way Arrow, Left-Right Arrow, Up-Down Arrow

**Stars (2):** 4-Point Star, 6-Point Star

**Callouts (3):** Rounded Rectangle Callout, Oval Callout, Cloud Callout

**Other:** Heart, Polygon (free-form)

All shapes rendered to temp layer during drag, committed via PatchCommand on release (same pattern as existing RectangleTool).

### Selection Tools

**Rectangular Selection** — existing RectSelectionTool, upgraded to floating selection model (see below).  
**Free-form Selection** — existing LassoSelectionTool, upgraded similarly.

---

## Floating Selection

Modern Paint selection behaviour replaces the current mask-only approach:

1. **Draw selection** — dashed animated "marching ants" outline around selected region.
2. **First drag** — pixels under selection are lifted into `FloatingSelection { image: RgbaImage, pos: Pos2 }` stored on `AppState`. The vacated area on the canvas is filled with Color 2. A PatchCommand captures this lift operation for undo.
3. **Drag floating selection** — move `FloatingSelection::pos`. The floating image is drawn as the temp layer overlay at its current position.
4. **Stamp (Ctrl+drag)** — copies floating selection without clearing the original (hold Ctrl while dragging).
5. **Deselect / commit** — floating image is composited onto canvas at final position. PatchCommand captures the stamp. `AppState::floating_selection` → None.
6. **Delete selection** — fill selected area with Color 2, no floating selection created.
7. **Arrow keys** — move floating selection 1px per press.

---

## Image Operations

### Resize / Skew (Ctrl+E)
`egui::Window` modal with:
- Resize section: Width + Height fields, "Maintain aspect ratio" checkbox, unit toggle (Pixels / Percentage)
- Skew section: Horizontal + Vertical angle fields (degrees)
- OK / Cancel buttons
- On OK: `ImageStore::resize()` (existing) extended to support skew via affine transform

### Rotate / Flip
Applied immediately (no dialog). Operations: 90° CW, 90° CCW, 180°, Flip Horizontal, Flip Vertical. Each creates a full-canvas PatchCommand for undo.

### Crop to Selection
Only enabled when a selection is active. Resizes canvas to selection bounding box. Creates PatchCommand.

### Invert Colors (Ctrl+Shift+I)
Per-pixel: `[255-r, 255-g, 255-b, a]`. Full-canvas PatchCommand.

---

## Color System

- **Color 1** (foreground): used by left mouse button for all tools
- **Color 2** (background): used by right mouse button; also used by Eraser and to fill vacated selection area
- **Palette**: 20 fixed colors matching modern Paint exactly (2 rows of 10)
- **Edit Colors**: opens egui color picker window; clicking OK updates Color 1 or Color 2 depending on which was last clicked
- **Eyedropper shortcut**: while any tool is active, holding `Alt` temporarily switches to Color Picker

---

## Clipboard

- **Copy** (Ctrl+C): copies selected region (or full canvas if no selection) into `AppState::clipboard: Option<RgbaImage>` and into the OS clipboard via egui's clipboard API
- **Cut** (Ctrl+X): copy + fill selection with Color 2
- **Paste** (Ctrl+V): creates a new floating selection from clipboard image, positioned at top-left of canvas

---

## Status Bar

Bottom panel (togglable via View tab). Three sections:

- **Left**: Cursor position `X, Y` px (updates on mouse move over canvas)
- **Center**: Selection size `W × H px` (shown only when selection active, else blank)
- **Right**: Canvas size `800×600px` | zoom percentage | zoom slider (drag or click)

---

## File Operations

| Action | Shortcut | Behaviour |
|---|---|---|
| New | Ctrl+N | Dialog: width + height inputs, default 800×600. Clears canvas, resets undo stack. |
| Open | Ctrl+O | File picker (PNG, JPEG, BMP, GIF, TIFF). Existing `ImageStore::from_file()`. |
| Save | Ctrl+S | Save to last path; if unsaved, falls through to Save As. |
| Save As | Ctrl+Shift+S | File picker. Saves composite via `ImageStore::save()`. |

---

## Keybindings (Modern Paint defaults)

| Action | Shortcut |
|---|---|
| Undo | Ctrl+Z |
| Redo | Ctrl+Y |
| New | Ctrl+N |
| Open | Ctrl+O |
| Save | Ctrl+S |
| Save As | Ctrl+Shift+S |
| Select All | Ctrl+A |
| Copy | Ctrl+C |
| Cut | Ctrl+X |
| Paste | Ctrl+V |
| Delete selection | Delete |
| Deselect / cancel | Escape |
| Resize dialog | Ctrl+E |
| Invert colors | Ctrl+Shift+I |
| Pencil | P |
| Brush | B |
| Eraser | E |
| Fill | F |
| Color Picker | I |
| Text | T |
| Magnifier | M |
| Airbrush | A |
| Rectangular Select | S |
| Zoom in | Ctrl+= |
| Zoom out | Ctrl+- |
| Zoom (scroll) | Ctrl+scroll wheel |
| Move selection | Arrow keys (1px) |
| Pan canvas | Space+drag or middle mouse |
| Constrain shape | Hold Shift while drawing |
| Alt eyedropper | Hold Alt while any tool active |

---

## What is NOT in Phase A

These are intentionally deferred to Phase D or later:

- Layer panel / multi-layer editing
- Blend modes
- Brush textures / custom brush stamps
- Pressure sensitivity
- Color mixing
- Rulers (View tab checkbox exists in UI but has no effect in Phase A — rendering deferred)
- Print
- Animated GIF export
- AI background removal (Win11 Paint feature)
- Transparent PNG alpha editing (can open transparent PNGs; editing alpha is Phase D)
