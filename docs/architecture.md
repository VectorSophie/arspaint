# ARSPaint v2 architecture

ARSPaint v2 is Pinta 3.1.2 plus an additive workflow layer. This document is
the map of where that layer lives and how it hooks into Pinta. See
`UPSTREAM.md` for the branch/remote layout and merge procedure, and
`docs/legacy-arspaint.md` for what the old Rust prototype tried and why it
isn't the base anymore.

## Principle: additive, not invasive

Pinta's own architecture (documents, layers, history, tools, effects,
add-ins, file formats) is untouched in its normal operation. ARSPaint code:

- lives under an `Ars` folder/namespace inside whichever existing project it
  most naturally belongs to (not a new assembly - see below for why),
- touches existing Pinta files only at small, named integration points,
- reuses Pinta's own systems (`BaseHistoryItem`/`DocumentHistory` for undo,
  `Command`/`Gio.SimpleAction` for actions, the `.ora` XML writer for
  persistence) instead of building parallel infrastructure.

No new assembly was created. `Pinta.Core.Ars.*` types live inside the
existing `Pinta.Core` project, and UI-facing ARS types live inside whichever
UI project already owns the widget they extend (`Pinta.Gui.Widgets`, not a
new `Pinta.Ars` project) so they can reference `Gtk`/`Gio` and the widgets
they hook into without a new project reference edge. `namespace Pinta.Ars`
is used consistently regardless of which physical project a file lives in -
the namespace is the module boundary, not the assembly.

## Implemented: semantic layer roles

The first workflow feature, and the template for how everything else in the
spec should be added.

**Model** - `Pinta.Core/Ars/LayerRole.cs`, `LayerRoleStore.cs`:

- `LayerRole` enum: `Generic | Reference | Ink | Color | Shade`.
- `LayerRoleStore` is a `ConditionalWeakTable<UserLayer, ...>` side-table,
  *not* a field added to `UserLayer`. A layer with no entry is `Generic`.
  This means marking a layer costs nothing for the overwhelming majority of
  layers that never get a role, and Pinta's own `UserLayer` class needed no
  changes at all.

**Undo** - `Pinta.Core/Ars/SetLayerRoleHistoryItem.cs`:

- A normal `BaseHistoryItem` (same base class as every other Pinta history
  item) capturing old/new role by layer index. Pushed through the existing
  `Document.History.PushNewItem`, so it participates in the regular
  undo/redo stack, gets undone/redone on Ctrl+Z/Ctrl+Y, and is visible in
  the History panel like any other action - no ARS-specific history UI.

**Actions** - `Pinta.Gui.Widgets/Ars/LayerRoleActions.cs`:

- Five `Pinta.Core.Command`s (`ars-role-generic/reference/ink/color/shade`),
  registered with the `Gtk.Application` the same way `LayerActions` registers
  its commands. `AppendRoleSubmenu(Gio.Menu)` builds a "Layer Role" submenu
  that any menu model can embed.

**UI hooks** (the only touches to existing Pinta files, both additive):

- `Pinta/MainWindow.cs`: two lines calling
  `Ars.LayerRoleActions.RegisterActions/RegisterHandlers` alongside the
  existing `PintaCore.Actions.*.RegisterActions` calls.
- `Pinta.Gui.Widgets/Widgets/Layers/LayersListViewItemWidget.cs`: the
  right-click context menu gets one extra `roleSection` built from
  `LayerRoleActions.AppendRoleSubmenu`; `LayersListViewItem.Label` prepends
  the role's badge (`[REF]`, `[INK]`, `[COL]`, `[SHD]`) when present. Because
  the badge is pushed through a real history item, the panel's existing
  history-driven refresh (`HandleHistoryChanged` -> `NotifyLayerModified`)
  redraws it with zero new refresh plumbing.

**Persistence** - `Pinta.Core/ImageFormats/OraFormat.cs`:

- Investigated OpenRaster's extension story first, per the spec's ordering.
  OpenRaster layers are plain XML elements with no schema validation on
  unknown attributes, so a role round-trips as a plain `arspaint-role="ink"`
  attribute on the `<layer>` element. A `Generic` layer writes no attribute
  at all, so a document with no ARS metadata is byte-identical to what
  upstream Pinta would write except for the role attribute on marked layers.
  Any other OpenRaster-compliant editor opening the file simply ignores the
  unrecognized attribute - normal `.ora` interoperability is not broken.

**What's deliberately not done yet** (see "Known issues" in the handoff
report): keyboard shortcuts for role assignment, the Layer Properties dialog
doesn't expose role, and export doesn't yet exclude Reference-role layers
(spec section 41) - that needs a dedicated flattening path and shouldn't be
grafted onto the general-purpose `GetFlattenedImage` used by merge/thumbnails/
`.ora` save.

## Integration points established for future ARS work

These are real, in this codebase now - not proposed:

| Concern | Where it hooks in |
|---|---|
| New undoable ARS action | Subclass `Pinta.Core.BaseHistoryItem`, push via `doc.History.PushNewItem` |
| New ARS command/keybinding | New `Pinta.Core.Command`, registered with `Gtk.Application.AddCommands` |
| Per-layer ARS metadata | `ConditionalWeakTable<UserLayer, T>` side-table, not a `UserLayer` field |
| `.ora` round-trip of ARS metadata | Extra attribute in `OraFormat.GetLayerXmlData` (write) / the layer-parsing loop in `Import` (read) |
| ARS UI in the layers panel | `LayersListViewItemWidget` (context menu, label) |
| App bootstrap / global registration | `Pinta/MainWindow.cs`, next to the existing `PintaCore.Actions.*.RegisterActions` calls |

## Implemented: dark mode uses VS Code's Abyss palette

`Pinta.Resources/Resources/style-abyss-dark.css` overrides libadwaita's
named colors (`window_bg_color`, `sidebar_bg_color`, `accent_bg_color`,
`theme_selected_bg_color`, etc.) with the exact values from VS Code's
built-in Abyss theme, so every stock widget picks it up for free.
`Pinta.Gui.Widgets/Ars/AbyssDarkTheme.cs` swaps it in/out in code by
watching `Adw.StyleManager`'s generic property-notify signal for `"dark"`
(there's no dedicated `NotifyDark` event in the GirCore binding - confirmed
by reflecting on the actual `Adw-1.dll`), since GTK CSS has no media-query
equivalent. Applies whenever the resolved scheme is dark, system-default or
user-forced.

## Implemented: basic command registry, ARS tool bindings, canvas-only mode

`Pinta.Core/Ars/ArsCommandRegistry.cs` is the "one command registry" the
spec calls for: a flat list of `(id, description, key, action)` entries
with a single `TryDispatchKey(Gdk.Key)` lookup. It intentionally carries no
count/context/repeatability metadata yet - add that when a which-key popup
or colon-command mode (neither built) actually needs it, not before.

Before adding any tool keybindings, the existing Pinta tool shortcuts
(`BaseTool.ShortcutKey`) were checked: `b`/`e`/`f` (brush/eraser/fill)
already match the spec's keyboard layer exactly. `i` and `v` don't - Pinta
already uses `K` (color picker) and `S` (shared by rectangle select, lasso,
and magic wand). Rather than reassign those and risk breaking existing
muscle memory, `i` and `v` are registered as ARS commands in
`Pinta.Gui.Widgets/Ars/ArsKeyboardLayer.cs`, dispatched in
`MainWindow.HandleGlobalKeyPress` only *after* Pinta's own
`ToolManager.SetCurrentTool(Gdk.Key)` check has had first refusal - `K` and
`S` are untouched.

The same file implements `Tab` as a canvas-only mode toggle, by reusing
Pinta's existing `ToolBar`/`ToolBox`/`ToolWindows` view-visibility toggle
actions (confirmed these apply live via their registered `Toggled` handlers,
unlike `MenuBar`, which requires an app restart - see
`MenuBarToggledAction.cs` - so `MenuBar` is deliberately left alone). The
status bar stays visible throughout, per spec section 31.

## Not yet built

Which-key popup, colon-command mode, dot-repeat, macros, reference workflow
(visibility/opacity/split view), direct canvas resize handles, ARS Pen/
Pixel Pencil tool presets, palette extraction, workflow presets, and smart
role inference are all unimplemented. The dense statusline format from spec
section 30 (`DRAW | Ink | ARS Pen 4px | #181818 | 127% | REF 35% |
1800x1800`) is also unimplemented - Pinta's existing status bar (cursor
position, selection bounds, zoom) was left alone rather than restructured
without a reliable way to visually verify the result in this session (see
`docs/testing.md`).

What's implemented (semantic layer roles, the Abyss dark palette, the
command registry + tool bindings + canvas-only mode) is also the worked
example for how the rest should be built: a small `Pinta.Core.Ars` model
(plus a `Pinta.Gui.Widgets/Ars` or `Pinta/Ars` piece when GTK/UI types are
needed), a thin hook into the relevant existing Pinta file, and reuse of
Pinta's own action/history/persistence/settings systems throughout - never
parallel infrastructure.
