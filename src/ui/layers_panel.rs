use super::ArsApp;
use eframe::egui::{self, Color32, Ui};
use crate::layers::Layer;

impl ArsApp {
    pub(super) fn render_layers_panel(&mut self, ui: &mut Ui) {
        ui.heading("Layers");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("＋").on_hover_text("Add raster layer").clicked() {
                let (w, h) = (self.state.image.width(), self.state.image.height());
                let n = self.state.image.layers.len() + 1;
                let cmd = self.state.image.edit("Add layer", |img| {
                    img.add_layer(Layer::new_raster(w, h, format!("Layer {}", n)));
                });
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }
            let can_delete = self.state.image.layers.len() > 1;
            if ui.add_enabled(can_delete, egui::Button::new("✕")).on_hover_text("Delete layer").clicked() {
                let idx = self.state.image.active_layer;
                let cmd = self.state.image.edit("Delete layer", |img| {
                    img.layers.remove(idx);
                    img.active_layer = idx.saturating_sub(1).min(img.layers.len() - 1);
                    img.mark_dirty();
                });
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }
            let can_up = self.state.image.active_layer + 1 < self.state.image.layers.len();
            if ui.add_enabled(can_up, egui::Button::new("↑")).on_hover_text("Move up").clicked() {
                let idx = self.state.image.active_layer;
                let cmd = self.state.image.edit("Move layer up", |img| {
                    img.layers.swap(idx, idx + 1);
                    img.active_layer = idx + 1;
                    img.mark_dirty();
                });
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }
            let can_down = self.state.image.active_layer > 0;
            if ui.add_enabled(can_down, egui::Button::new("↓")).on_hover_text("Move down").clicked() {
                let idx = self.state.image.active_layer;
                let cmd = self.state.image.edit("Move layer down", |img| {
                    img.layers.swap(idx, idx - 1);
                    img.active_layer = idx - 1;
                    img.mark_dirty();
                });
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }
            if ui.button("⎘").on_hover_text("Duplicate layer").clicked() {
                let idx = self.state.image.active_layer;
                let cmd = self.state.image.edit("Duplicate layer", |img| {
                    let clone = img.layers[idx].clone();
                    img.layers.insert(idx + 1, clone);
                    img.active_layer = idx + 1;
                    img.mark_dirty();
                });
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }
            let can_merge = self.state.image.active_layer > 0;
            if ui.add_enabled(can_merge, egui::Button::new("⇩")).on_hover_text("Merge down").clicked() {
                let cmd = self.state.image.edit("Merge down", |img| img.merge_down());
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }
        });

        ui.separator();

        let n = self.state.image.layers.len();
        let indices: Vec<usize> = (0..n).rev().collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for idx in indices {
                let is_active = idx == self.state.image.active_layer;

                let frame_color = if is_active {
                    Color32::from_rgb(40, 52, 87)
                } else {
                    Color32::TRANSPARENT
                };

                egui::Frame::none().fill(frame_color).inner_margin(4.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Visibility eye
                        let vis = self.state.image.layers[idx].visible;
                        let eye = if vis { "👁" } else { "  " };
                        if ui.small_button(eye).clicked() {
                            self.state.image.layers[idx].visible = !vis;
                            self.state.image.mark_dirty();
                            self.image_dirty = true;
                        }

                        // Alpha lock
                        let al = self.state.image.layers[idx].alpha_locked;
                        if ui.selectable_label(al, "🔒").on_hover_text("Lock alpha").clicked() {
                            self.state.image.layers[idx].alpha_locked = !al;
                        }

                        // Clip to below
                        let clip = self.state.image.layers[idx].clipped;
                        if ui.selectable_label(clip, "🖇").on_hover_text("Clip to layer below").clicked() {
                            self.state.image.layers[idx].clipped = !clip;
                            self.state.image.mark_dirty();
                            self.image_dirty = true;
                        }

                        let name = self.state.image.layers[idx].name.clone();
                        if ui.selectable_label(is_active, &name).clicked() {
                            self.state.image.active_layer = idx;
                        }
                    });

                    if is_active {
                        ui.horizontal(|ui| {
                            ui.label("Opacity");
                            let mut op = self.state.image.layers[idx].opacity;
                            if ui.add(egui::Slider::new(&mut op, 0.0..=1.0).show_value(false)).changed() {
                                self.state.image.layers[idx].opacity = op;
                                self.state.image.mark_dirty();
                                self.image_dirty = true;
                            }
                            ui.label(format!("{:.0}%", self.state.image.layers[idx].opacity * 100.0));
                        });

                        let mut blend = self.state.image.layers[idx].blend;
                        egui::ComboBox::from_id_salt(format!("blend_{}", idx))
                            .selected_text(format!("{:?}", blend))
                            .show_ui(ui, |ui| {
                                use crate::layers::BlendMode;
                                for mode in [
                                    BlendMode::Normal, BlendMode::Multiply, BlendMode::Add,
                                    BlendMode::Screen, BlendMode::Overlay, BlendMode::SoftLight,
                                    BlendMode::Difference,
                                ] {
                                    if ui.selectable_value(&mut blend, mode, format!("{:?}", mode)).clicked() {
                                        self.state.image.layers[idx].blend = blend;
                                        self.state.image.mark_dirty();
                                        self.image_dirty = true;
                                    }
                                }
                            });
                    }
                });
            }
        });
    }
}
