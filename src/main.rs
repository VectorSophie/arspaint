mod commands;
mod image_store;
mod state;
mod tools;
mod ui;

use eframe::egui::IconData;
use std::sync::Arc;

fn load_icon() -> Option<Arc<IconData>> {
    let icon_bytes = include_bytes!("logo.png");
    match image::load_from_memory(icon_bytes) {
        Ok(image) => {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some(Arc::new(IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            }))
        }
        Err(e) => {
            log::warn!("Failed to load icon: {}", e);
            None
        }
    }
}

fn main() {
    env_logger::init();

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("ArsPaint - Rust Native")
            .with_icon(icon.unwrap_or_default()), // Use loaded icon or default (none)
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "ArsPaint",
        options,
        Box::new(|cc| Ok(Box::new(ui::ArsApp::new(cc)))),
    ) {
        log::error!("Failed to start application: {}", e);
    }
}
