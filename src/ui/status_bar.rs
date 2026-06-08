use super::ArsApp;
use eframe::egui::{self, Ui};

impl ArsApp {
    pub(super) fn render_status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if let Some(pos) = self.cursor_pos {
                let x = pos.x.max(0.0) as u32;
                let y = pos.y.max(0.0) as u32;
                ui.label(format!("{}, {}px", x, y));
                ui.separator();
            }
            ui.label(format!("{}×{}px", self.state.image.width(), self.state.image.height()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::Slider::new(&mut self.zoom, 0.1..=16.0).show_value(false).step_by(0.1));
                ui.label(format!("{:.0}%", self.zoom * 100.0));
            });
        });
    }
}
