use super::ArsApp;
use super::theme;
use eframe::egui::{self, Ui, Vec2};

impl ArsApp {
    pub(super) fn render_menu_bar(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            // Branding: logo + title
            if let Some(tex) = &self.logo_texture {
                ui.add(egui::Image::new((tex.id(), Vec2::splat(18.0))));
            }
            let title = self.state.current_save_path.as_ref()
                .and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string());
            ui.label(egui::RichText::new(format!("{}  —  ArsPaint", title)).color(theme::MUTED));
            ui.separator();

            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() { self.show_new_dialog = true; ui.close_menu(); }
                if ui.button("Open…").clicked() { self.do_open(); ui.close_menu(); }
                if ui.button("Save").clicked() { self.do_save(); ui.close_menu(); }
                if ui.button("Save As…").clicked() { self.do_save_as(); ui.close_menu(); }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() { self.state.command_stack.undo(&mut self.state.image); self.image_dirty = true; ui.close_menu(); }
                if ui.button("Redo").clicked() { self.state.command_stack.redo(&mut self.state.image); self.image_dirty = true; ui.close_menu(); }
                ui.separator();
                if ui.button("Cut").clicked() { self.do_cut(); ui.close_menu(); }
                if ui.button("Copy").clicked() { self.do_copy(); ui.close_menu(); }
                if ui.button("Paste").clicked() { self.do_paste(); ui.close_menu(); }
                if ui.button("Select All").clicked() { self.do_select_all(); ui.close_menu(); }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_grid, "Gridlines");
                ui.checkbox(&mut self.show_status_bar, "Status bar");
                ui.checkbox(&mut self.show_layers_panel, "Right dock");
            });
            ui.menu_button("Image", |ui| {
                if ui.button("Resize…").clicked() {
                    self.resize_w = self.state.image.width();
                    self.resize_h = self.state.image.height();
                    self.show_resize_dialog = true; ui.close_menu();
                }
                if ui.button("Crop to selection").clicked() { self.do_crop(); ui.close_menu(); }
                ui.separator();
                if ui.button("Rotate 90° right").clicked() { self.state.image.rotate90_cw(); self.image_dirty = true; ui.close_menu(); }
                if ui.button("Rotate 90° left").clicked() { self.state.image.rotate90_ccw(); self.image_dirty = true; ui.close_menu(); }
                if ui.button("Rotate 180°").clicked() { self.state.image.rotate180(); self.image_dirty = true; ui.close_menu(); }
                if ui.button("Flip horizontal").clicked() { self.state.image.flip_horizontal(); self.image_dirty = true; ui.close_menu(); }
                if ui.button("Flip vertical").clicked() { self.state.image.flip_vertical(); self.image_dirty = true; ui.close_menu(); }
                ui.separator();
                if ui.button("Invert colors").clicked() { self.state.image.invert_colors(); self.image_dirty = true; ui.close_menu(); }
            });
            ui.menu_button("Layers", |ui| {
                if ui.button("Add layer").clicked() {
                    let (w, h) = (self.state.image.width(), self.state.image.height());
                    let n = self.state.image.layers.len() + 1;
                    self.state.image.add_layer(crate::layers::Layer::new_raster(w, h, format!("Layer {}", n)));
                    self.image_dirty = true; ui.close_menu();
                }
            });
            ui.add_enabled(false, egui::Button::new("Adjustments"));
            ui.add_enabled(false, egui::Button::new("Effects"));
            ui.menu_button("Help", |ui| { ui.label("ArsPaint — Paint.NET-style edition"); });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let _ = ui.button("⚙");
            });
        });
    }
}
