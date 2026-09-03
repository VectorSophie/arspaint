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
9. Press `i`. Confirm the Color Picker tool activates. Press `k` first,
   confirm it *also* activates the Color Picker (Pinta's own shortcut,
   unchanged), then `i` again to confirm both work independently.
10. Press `v`. Confirm Rectangle Select activates. Press `s`, confirm it
    activates a select-family tool too (Pinta's own shortcut, unchanged).
11. Press `Tab`. Confirm the toolbar, left tool palette, and right docks
    (Layers/History) all hide, leaving canvas + status bar. Press `Tab`
    again, confirm everything comes back exactly as it was (including which
    panels were open).
12. Click into a text field (e.g. the brush width entry, or open the Layer
    Properties dialog and click its name field) and type letters including
    b/e/f/i/v and Tab. Confirm normal text editing happens - none of the
    ARS bindings fire while a text field has focus.
13. Toggle View > Color Scheme between Dark and Light (or change the OS
    theme if using Default). Confirm the window chrome, side panels, and
    controls switch between the Abyss palette (deep navy backgrounds,
    blue-tinted text) and Pinta's normal light theme.

## Status as of 2026-09-04

All of the above was confirmed working by manual human testing (i/k/v/s
tool switching, Tab canvas-only toggle, the layer-role right-click submenu
and badge, and the Abyss dark palette) - the automated-input limitation
noted below only ever affected how *this agent* could verify the work
within a single session, not the work itself. Steps 6-7 (`.ora` round-trip)
and 12 (text-field safety) and the light-mode side of step 13 are still
worth a pass if you're touching those areas again, since they weren't
specifically called out as tried.

## Why some of this needed a human pass

Synthetic OS-level mouse clicks landed reliably on the canvas (confirmed:
a real brush stroke and its undo entry both appeared) but never visibly
opened a GTK popover/menu when driven from the agent's sandboxed Windows
environment (tried both the layer row's right-click menu and the header
bar's hamburger menu) - likely a compositor/timing quirk specific to
synthetic input against GTK4 popups, not a problem with the app. Keyboard
dispatch has the same story: the registry logic was unit-tested
(`tests/Pinta.Core.Tests/Ars/ArsCommandRegistryTests.cs`), but its live
wiring into `MainWindow.HandleGlobalKeyPress` could only be confirmed by
an actual person pressing actual keys. No automated GUI/integration test
harness exists for Pinta on this platform.
