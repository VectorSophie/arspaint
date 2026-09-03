# Upstream relationship

ARSPaint is a fork of [Pinta](https://github.com/PintaProject/Pinta), a GTK
image editor descended from Paint.NET 3.0. ARSPaint keeps almost all of
Pinta and adds a workflow layer on top (see `docs/architecture.md`). It does
not claim authorship of Pinta or Paint.NET's original code — see
`license-mit.txt` and `license-pdn.txt`, both preserved unchanged at the repo
root.

## Base revision

- Upstream project: `PintaProject/Pinta`
- Base tag: `3.1.2` (latest stable at the time of the fork; verified against
  upstream tags, newer than the `3.1` release the original brief called out)
- Remote name: `pinta-upstream` (`https://github.com/PintaProject/Pinta.git`)

## Branch layout

```
master, legacy-rust (tag arspaint-v1-rust)   <- old Rust/egui prototype, frozen
pinta-upstream/3.1.2 (and full Pinta history) <- vendored via the remote
arspaint-v2                                   <- active development, rooted at 3.1.2
```

`arspaint-v2` shares no merge-base with `master`/`legacy-rust` — it is
rooted directly at Pinta's `3.1.2` tag, so upstream commit ancestry is
preserved intact rather than squashed into an import commit. `master` stays
untouched as a pointer to the old Rust HEAD; the actively developed branch
going forward is `arspaint-v2`.

## Where ARSPaint-specific code lives

New code goes in `Pinta.Ars` types/namespaces alongside the project they
extend (e.g. `Pinta.Core.Ars.*` for a new manager, `Pinta.Ars.*` for a
standalone assembly if one becomes necessary). Existing Pinta files are
touched only at clearly-marked integration points — a hook call, a menu item
registration, an extra field read at load time — never large in-place
rewrites. See `docs/architecture.md` for the current integration points.

## Fetching upstream updates

```bash
git fetch pinta-upstream --tags
git switch arspaint-v2
git merge <new-pinta-tag>       # e.g. git merge 3.1.3
```

Merge, not rebase: `arspaint-v2` is a long-lived branch other people/history
may build on, and Pinta's own history has merge commits, so rebasing it onto
each new tag would rewrite public history and false-linearize things that
aren't linear. A merge conflict here is upstream's actual line-level change
overlapping ours — worth seeing.

Prefer merging a tagged release over `pinta-upstream/master`, so drift only
happens at Pinta's own release cadence.

## Likely conflict hotspots

Ranked by how much ARS code will touch them:

1. **`Pinta/MainWindow.cs`, `Pinta/Main.cs`** — app bootstrap; ARS hooks in
   for keyboard dispatch, the leader-key/which-key layer, and the statusline
   will land here. Keep the diff to additive hook calls, not restructuring.
2. **`Pinta.Core/Layers/UserLayer.cs`** (or wherever layer metadata lives) —
   semantic layer roles need a field here or a side-table keyed by layer.
3. **`Pinta.Core/ImageFormats/OraFormat.cs`** — round-tripping ARS metadata
   (layer roles, reference settings) through `.ora` if the OpenRaster
   extension mechanism supports it; otherwise a sidecar file.
4. **`Pinta.Core/Managers/WorkspaceManager.cs`** — reference-layer awareness
   for export (excluding Reference-role layers) plugs in here.
5. **`Pinta.Tools/`** — `ARS Pen` / `Pixel Pencil` are new tool classes here,
   additive, low conflict risk.

Everything else in Pinta (effects, adjustments, most tools, file formats,
add-ins) should need zero ARS-side changes and merges from upstream cleanly.
