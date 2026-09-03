# Manual smoke test

Automated coverage is `Pinta.Core.Tests` and `Pinta.Effects.Tests`
(`dotnet test tests/Pinta.Core.Tests`, `dotnet test tests/Pinta.Effects.Tests`).
This is the human pass on top of that - what to actually click through after
a change, especially to anything under `Ars/` or any file this touches per
`docs/architecture.md`.

Windows-specific setup (skip on Linux/macOS, see `readme.md`): the GTK4/
libadwaita runtime DLLs come from MSYS2's CLANG64 environment, so
`clang64/bin` must be on `PATH` when running or testing, e.g.:

```
export PATH="/c/msys64/clang64/bin:$PATH"
dotnet run --project Pinta -p:Platform="Any CPU"
```

## Script

1. Launch the app. Confirm the window title reads `Unsaved Image 1 -
   ARSPaint` (not `Pinta`) and Help > About shows "ARSPaint" crediting
   Pinta.
2. Draw a stroke with the default brush. Confirm it appears on canvas and
   an entry shows up in the History panel; Ctrl+Z removes it.
3. Add a new layer (Layers panel toolbar, first icon). Right-click the new
   layer row -> "Layer Role" submenu -> pick "Reference". Confirm the row's
   label gets a `[REF]` prefix.
4. Ctrl+Z. Confirm the `[REF]` badge disappears (role change is undoable)
   and Ctrl+Shift+Z (or Edit > Redo) brings it back.
5. Repeat step 3 for Ink, Color, and Shade - confirm `[INK]`, `[COL]`,
   `[SHD]` respectively, and that picking "Generic" clears the badge again.
6. Save as `.ora` (File > Save As), close the document, reopen it. Confirm
   the role badge is still there on the layer you marked - this exercises
   the `arspaint-role` OpenRaster attribute round-trip.
7. Open that same `.ora` file in a *different* OpenRaster-compatible editor
   if one is available (or inspect the zip: `layer.xml` inside should show
   `arspaint-role="reference"` etc. as an ordinary XML attribute with no
   namespace declaration needed) - confirm it opens with no error and no
   visible corruption, i.e. an unrecognized attribute degrades gracefully.
8. Exercise a few ordinary Pinta operations unrelated to ARS - flip a layer,
   run an adjustment (e.g. Hue/Saturation), use the rectangle select tool,
   undo/redo through a mix of ARS and non-ARS history items in one stack.
   Confirm nothing about the ARS additions breaks normal Pinta usage.

## Known gaps at the end of this session

- Steps 3-7 were implemented and unit-tested (`tests/Pinta.Core.Tests/Ars/
  LayerRoleTests.cs`) but not confirmed end-to-end via the live GUI in this
  session - synthetic OS-level mouse input into the GTK4 window proved
  unreliable in this sandboxed Windows environment (likely a DPI-scaling
  mismatch between the injected coordinates and what GTK receives), so the
  right-click context menu specifically needs a human pass. Steps 1-2
  (branding, core paint/undo pipeline) *were* confirmed via screenshot.
- No automated GUI/integration test harness exists for Pinta on this
  platform; all `Ars` coverage so far is at the model/logic level
  (`Pinta.Core.Tests`), not through simulated UI interaction.
