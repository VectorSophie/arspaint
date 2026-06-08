use super::ArsApp;
use eframe::egui::{self, Context};
use crate::image_store::ImageStore;
use crate::tools::{LassoSelectionTool, RectSelectionTool};

impl ArsApp {
    pub(super) fn render_dialogs(&mut self, ctx: &Context) {
        // Rotate dropdown (simple window)
        if self.show_rotate_menu {
            egui::Window::new("Rotate / Flip").collapsible(false).resizable(false)
                .show(ctx, |ui| {
                    if ui.button("Rotate 90° right").clicked()  { self.state.image.rotate90_cw();  self.image_dirty = true; self.show_rotate_menu = false; }
                    if ui.button("Rotate 90° left").clicked()   { self.state.image.rotate90_ccw(); self.image_dirty = true; self.show_rotate_menu = false; }
                    if ui.button("Rotate 180°").clicked()       { self.state.image.rotate180();    self.image_dirty = true; self.show_rotate_menu = false; }
                    ui.separator();
                    if ui.button("Flip horizontal").clicked()   { self.state.image.flip_horizontal(); self.image_dirty = true; self.show_rotate_menu = false; }
                    if ui.button("Flip vertical").clicked()     { self.state.image.flip_vertical();   self.image_dirty = true; self.show_rotate_menu = false; }
                    if ui.button("Cancel").clicked() { self.show_rotate_menu = false; }
                });
        }

        // Select dropdown
        if self.show_select_menu {
            let img_w = self.state.image.width();
            let img_h = self.state.image.height();
            egui::Window::new("Select").collapsible(false).resizable(false)
                .show(ctx, |ui| {
                    if ui.button("Rectangular Selection").clicked() {
                        self.state.active_tool = Box::new(RectSelectionTool::new());
                        self.show_select_menu = false;
                    }
                    if ui.button("Free-form Selection").clicked() {
                        self.state.active_tool = Box::new(LassoSelectionTool::new());
                        self.show_select_menu = false;
                    }
                    if ui.button("Select All").clicked() {
                        // fill entire selection mask
                        let mut mask = image::GrayImage::new(img_w, img_h);
                        for p in mask.pixels_mut() { *p = image::Luma([255]); }
                        self.state.image.selection = Some(mask);
                        self.show_select_menu = false;
                    }
                    if ui.button("Cancel").clicked() { self.show_select_menu = false; }
                });
        }

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
