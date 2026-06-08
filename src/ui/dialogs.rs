use super::ArsApp;
use eframe::egui::{self, Context};
use crate::image_store::ImageStore;

impl ArsApp {
    pub(super) fn render_dialogs(&mut self, ctx: &Context) {
        // Resize dialog
        if self.show_resize_dialog {
            let orig_w = self.state.image.width();
            let orig_h = self.state.image.height();
            egui::Window::new("Resize").collapsible(false).resizable(false)
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.resize_pct, "Percentage");
                    ui.checkbox(&mut self.resize_lock_aspect, "Maintain aspect ratio");
                    ui.separator();
                    if self.resize_pct {
                        let mut pct_w = self.resize_w as f32 / orig_w as f32 * 100.0;
                        let mut pct_h = self.resize_h as f32 / orig_h as f32 * 100.0;
                        ui.horizontal(|ui| {
                            ui.label("Width %:");
                            if ui.add(egui::DragValue::new(&mut pct_w).range(1.0..=800.0)).changed() {
                                self.resize_w = (orig_w as f32 * pct_w / 100.0) as u32;
                                if self.resize_lock_aspect {
                                    self.resize_h = (orig_h as f32 * pct_w / 100.0) as u32;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Height %:");
                            if ui.add(egui::DragValue::new(&mut pct_h).range(1.0..=800.0)).changed() {
                                self.resize_h = (orig_h as f32 * pct_h / 100.0) as u32;
                                if self.resize_lock_aspect {
                                    self.resize_w = (orig_w as f32 * pct_h / 100.0) as u32;
                                }
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label("Width px:");
                            if ui.add(egui::DragValue::new(&mut self.resize_w).range(1..=8000_u32)).changed() {
                                if self.resize_lock_aspect {
                                    self.resize_h = (orig_h as f32 * self.resize_w as f32 / orig_w as f32) as u32;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Height px:");
                            if ui.add(egui::DragValue::new(&mut self.resize_h).range(1..=8000_u32)).changed() {
                                if self.resize_lock_aspect {
                                    self.resize_w = (orig_w as f32 * self.resize_h as f32 / orig_h as f32) as u32;
                                }
                            }
                        });
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            let (w, h) = (self.resize_w.max(1), self.resize_h.max(1));
                            self.state.image.resize_scaled(w, h);
                            self.image_dirty = true;
                            self.show_resize_dialog = false;
                        }
                        if ui.button("Cancel").clicked() { self.show_resize_dialog = false; }
                    });
                });
        }

        // New file dialog
        if self.show_new_dialog {
            egui::Window::new("New Image").collapsible(false).resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(&mut self.new_w).range(1..=8000_u32));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(&mut self.new_h).range(1..=8000_u32));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            self.state.image = ImageStore::new(self.new_w, self.new_h);
                            self.state.command_stack = crate::commands::CommandStack::new();
                            self.state.current_save_path = None;
                            self.base_texture = None;
                            self.image_dirty = true;
                            self.show_new_dialog = false;
                        }
                        if ui.button("Cancel").clicked() { self.show_new_dialog = false; }
                    });
                });
        }
    }
}
