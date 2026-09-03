
# ARSPaint

ARSPaint is a drawing editor built around direct canvas interaction,
reference-aware layers, fast keyboard commands, workflow presets, and simple
color extraction. **ARSPaint is based on [Pinta](https://www.pinta-project.com/)**
and keeps almost all of it - normal layers, effects, adjustments, selection
tools, text, shapes, gradients, blend modes, history, file formats, add-ins -
plus an additive ARS-specific workflow layer on top. Someone who ignores the
ARS features entirely still has a capable, ordinary Pinta.

The workflow this is built for: bring in a reference image, draw lineart
with one predictable default pen, optionally color and shade it, export.
The interaction principle is that the mouse/stylus does spatial work and the
keyboard does everything else - tool switching, layer state, view, presets.

See `UPSTREAM.md` for the exact Pinta revision this is based on and how to
pull in upstream updates, `docs/architecture.md` for how ARS code is
organized and where it hooks into Pinta, and `docs/legacy-arspaint.md` for
why this isn't the original Rust/egui ARSPaint prototype (preserved on the
`legacy-rust` branch).

## Status

Early. Semantic layer roles (`Reference`/`Ink`/`Color`/`Shade`, undoable,
`.ora`-persisted) are implemented; see `docs/architecture.md` for what's
built and what's still just in the spec.

## Attribution and licensing

Pinta is Copyright (C) 2010 Jonathan Pobst and contributors, MIT-licensed
(`license-mit.txt`); it also incorporates MIT-licensed code from Paint.NET
3.36 (`license-pdn.txt`). ARSPaint does not claim authorship of Pinta or
Paint.NET - it's a fork with an additional workflow layer credited in the
About dialog and here.

- [Paint.NET 3.0](http://www.getpaint.net/) icons, MIT License
- [Silk icon set](https://github.com/markjames/famfamfam-silk-icons), CC BY 3.0
- [Fugue icon set](https://p.yusukekamiyamane.com), CC BY 3.0
- Pinta contributors' icons (see `Pinta.Resources/icons/pinta-icons.md`)

## Building on Windows

First, install the required GTK-related dependencies:
- Install [MSYS2](https://www.msys2.org)
- From the CLANG64 terminal, run `pacman -S mingw-w64-clang-x86_64-libadwaita mingw-w64-clang-x86_64-webp-pixbuf-loader`.
  - For ARM64 Windows, use the `CLANGARM64` terminal and replace `clang-x86_64` with `clang-aarch64`.

Pinta can then be built by opening `Pinta.sln` in [Visual Studio](https://visualstudio.microsoft.com/).
Ensure that .NET 8 is installed via the Visual Studio installer.

For building on the command line, `clang64/bin` needs to be on `PATH` at
run/test time so the app can find the GTK4/libadwaita DLLs:
- [Install the .NET 8 SDK](https://dotnet.microsoft.com/).
- Build:
  - `dotnet build Pinta.sln -p:Platform="Any CPU"`
- Run:
  - `export PATH="/c/msys64/clang64/bin:$PATH"` (adjust for your MSYS2 install location)
  - `dotnet run --project Pinta -p:Platform="Any CPU"`
- Test:
  - `dotnet test tests/Pinta.Core.Tests/Pinta.Core.Tests.csproj -c Debug`
  - `dotnet test tests/Pinta.Effects.Tests/Pinta.Effects.Tests.csproj -c Debug`

## Building on macOS

- Install .NET 8 and GTK4
  - `brew install dotnet-sdk libadwaita adwaita-icon-theme gettext webp-pixbuf-loader`
  - For Apple Silicon, set `DYLD_LIBRARY_PATH=/opt/homebrew/lib` in the environment so that Pinta can load the GTK libraries
  - For Intel, you may need to set `DYLD_LIBRARY_PATH=/usr/local/lib` when using .NET 9 or higher
- Build:
  - `dotnet build`
- Run:
  - `dotnet run --project Pinta`

## Building on Linux

- Install [.NET 8](https://dotnet.microsoft.com/) following the instructions for your Linux distribution.
- Install other dependencies (instructions are for Ubuntu 22.10, but should be similar for other distros):
  - `sudo apt install autotools-dev autoconf-archive gettext intltool libadwaita-1-dev`
  - Minimum library versions: `gtk` >= 4.18 and `libadwaita` >= 1.7
  - Optional dependencies: `webp-pixbuf-loader`
- Build (option 1, for development and testing):
  - `dotnet build`
  - `dotnet run --project Pinta`
- Build (option 2, for installation):
  - `./autogen.sh`
    - If building from a tarball, run `./configure` instead.
    - Add the `--prefix=<install directory>` argument to install to a directory other than `/usr/local`.
  - `make install`

## Contributing to ARSPaint

This is a personal fork; see `docs/architecture.md` before adding ARS
features and `UPSTREAM.md` before merging a new Pinta release. For anything
about Pinta itself (not the ARS layer), upstream's own channels apply:

- [Technical help](https://github.com/PintaProject/Pinta/discussions)
- [Bugs/issues](https://github.com/PintaProject/Pinta/issues)
- [Pinta CHANGELOG](https://github.com/PintaProject/Pinta/blob/master/CHANGELOG.md)
- `patch-guidelines.md` in this repo, for patching conventions

## Code signing policy

Inherited from upstream Pinta:
- Free code signing on Windows provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/).
- Privacy policy: this program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.
