# ArsPaint

A native desktop clone of MS Paint written in Rust, designed for performance, correctness, and maintainability. 

Built with **egui** for the interface and **image** for pixel manipulation.

![ArsPaint](https://dummyimage.com/800x600/1a1b26/ffffff&text=ArsPaint+Preview)

## Features

- **Infinite Canvas**: Zoom (`Ctrl + Scroll`) and Pan (`Middle Mouse` or `Space + Drag`) freely.
- **Tools**:
  - **Brush**: Variable size, instant response.
  - **Eraser**: Transparent erasing support.
  - **Line**: Drag-to-draw straight lines.
- **Robust Undo/Redo**: 
  - Command-based architecture.
  - Memory-efficient "patch" storage (saves only changed pixels).
- **File Support**: Open and Save PNG, JPG, and BMP files.
- **Dark Mode**: Uses the "Tokyonight" color scheme by default.

## Prerequisites

To build ArsPaint on Windows, you **must** have the MSVC toolchain installed.

1.  **Install Rust**: [rustup.rs](https://rustup.rs/)
2.  **Install C++ Build Tools**:
    -   Download the [Visual Studio Installer](https://visualstudio.microsoft.com/downloads/).
    -   Select **"Desktop development with C++"**.
    -   Ensure the **Windows 10/11 SDK** is checked.

*Note: GNU/MinGW toolchains may work but are not officially supported due to linker complexities with some dependencies.*

## Build & Run

```bash
# Clone the repository
git clone https://github.com/yourusername/arspaint.git
cd arspaint

# Run in release mode (recommended for performance)
cargo run --release
```

## Controls

| Action | Control |
|--------|---------|
| **Draw** | Left Mouse Button |
| **Pan Canvas** | Middle Mouse Button OR Space + Drag |
| **Zoom** | Ctrl + Mouse Wheel |
| **Undo** | Ctrl + Z (or UI Button) |
| **Redo** | Ctrl + Y (or UI Button) |
| **Change Size** | Drag "Size" value in toolbar |

## Architecture

ArsPaint follows a strict ownership model to avoid global mutable state:

-   **`main.rs`**: Entry point and window setup.
-   **`ui.rs`**: Handles rendering the `egui` interface and input events.
-   **`state.rs`**: Container for application state (Image, CommandStack, Active Tool).
-   **`image_store.rs`**: Wrapper around `image::RgbaImage` for safe raw pixel access.
-   **`tools.rs`**: Trait-based tool system.
    -   Tools implement `update()` to modify a temporary layer.
    -   On commit (mouse release), tools return a `Command` struct.
-   **`commands.rs`**: Implements the Command Pattern.
    -   `PatchCommand` stores the "before" and "after" image sub-regions for undo/redo.

## License

MIT
