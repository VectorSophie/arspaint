use super::ArsApp;
use crate::state::PALETTE;
use eframe::egui::{self, Color32, Sense, Ui, Vec2};
use image::Rgba;

impl ArsApp {
    pub(super) fn render_colors_panel(&mut self, ui: &mut Ui) {
        // Color 1 / Color 2 + swap
        ui.horizontal(|ui| {
            let c1 = self.state.color1;
            let mut c1_arr = [c1[0], c1[1], c1[2]];
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("1").small());
                if ui.color_edit_button_srgb(&mut c1_arr).changed() {
                    self.state.color1 = Rgba([c1_arr[0], c1_arr[1], c1_arr[2], 255]);
                }
            });
            let c2 = self.state.color2;
            let mut c2_arr = [c2[0], c2[1], c2[2]];
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("2").small());
                if ui.color_edit_button_srgb(&mut c2_arr).changed() {
                    self.state.color2 = Rgba([c2_arr[0], c2_arr[1], c2_arr[2], 255]);
                }
            });
            if ui.button("⇄").on_hover_text("Swap").clicked() {
                std::mem::swap(&mut self.state.color1, &mut self.state.color2);
            }
        });

        // HSV wheel for Color 1
        let c1 = self.state.color1;
        let mut c1_32 = Color32::from_rgba_unmultiplied(c1[0], c1[1], c1[2], 255);
        if egui::color_picker::color_picker_color32(ui, &mut c1_32, egui::color_picker::Alpha::Opaque) {
            let [r, g, b, _] = c1_32.to_array();
            self.state.color1 = Rgba([r, g, b, 255]);
        }

        // 20-swatch palette
        ui.horizontal_wrapped(|ui| {
            for color in &PALETTE {
                let c32 = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 255);
                let (rect, resp) = ui.allocate_at_least(Vec2::splat(14.0), Sense::click());
                ui.painter().rect_filled(rect, 1.0, c32);
                ui.painter().rect_stroke(rect, 1.0, egui::Stroke::new(0.5, Color32::from_gray(60)));
                if resp.clicked() { self.state.color1 = *color; }
                if resp.secondary_clicked() { self.state.color2 = *color; }
            }
        });
    }
}
