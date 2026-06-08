use super::ArsApp;
use crate::state::FillMode;
use eframe::egui::{self, Ui};

impl ArsApp {
    pub(super) fn render_toolbar(&mut self, ui: &mut Ui) {
        // Rank 1 — common commands
        ui.horizontal(|ui| {
            if ui.button("New").clicked() { self.show_new_dialog = true; }
            if ui.button("Open").clicked() { self.do_open(); }
            if ui.button("Save").clicked() { self.do_save(); }
            ui.separator();
            if ui.button("↶ Undo").clicked() { self.state.command_stack.undo(&mut self.state.image); self.image_dirty = true; }
            if ui.button("↷ Redo").clicked() { self.state.command_stack.redo(&mut self.state.image); self.image_dirty = true; }
            ui.separator();
            if ui.button("Cut").clicked() { self.do_cut(); }
            if ui.button("Copy").clicked() { self.do_copy(); }
            if ui.button("Paste").clicked() { self.do_paste(); }
        });
        ui.separator();
        // Rank 2 — active tool options (delegates to the tool's own configure())
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}:", self.state.active_tool.name())).strong());
            self.state.active_tool.configure(ui, &mut self.state.tool_settings);
            // Shape tools carry a Fill mode that configure() does not render.
            let is_shape = self.state.active_tool.active_shape_kind().is_some()
                || matches!(self.state.active_tool.name(), "Rectangle" | "Ellipse");
            if is_shape {
                ui.separator();
                ui.label("Fill:");
                for (mode, label) in [(FillMode::Outline, "Outline"), (FillMode::Fill, "Fill"), (FillMode::Both, "Both")] {
                    if ui.selectable_label(self.state.tool_settings.shape_fill_mode == mode, label).clicked() {
                        self.state.tool_settings.shape_fill_mode = mode;
                    }
                }
            }
        });
    }
}
