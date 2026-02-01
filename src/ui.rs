use crate::state::AppState;
use crate::tools::ToolInput;
use eframe::egui::{
    self, Color32, Context, PointerButton, Pos2, Rect, ScrollArea, Sense, TextureOptions, Ui, Vec2,
};
use eframe::Frame;
use image::{Rgba, RgbaImage};

pub struct ArsApp {
    state: AppState,
    base_texture: Option<egui::TextureHandle>,
    layer_texture: Option<egui::TextureHandle>,
    zoom: f32,
    pan: Vec2,
    image_dirty: bool,
}

impl ArsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Tokyonight Theme
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = Color32::from_rgb(26, 27, 38); // #1a1b26
        visuals.panel_fill = Color32::from_rgb(26, 27, 38);
        visuals.extreme_bg_color = Color32::from_rgb(22, 22, 30);
        cc.egui_ctx.set_visuals(visuals);

        Self {
            state: AppState::new(800, 600),
            base_texture: None,
            layer_texture: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            image_dirty: true,
        }
    }

    fn update_textures(&mut self, ctx: &Context) {
        // Update base texture if dirty
        if self.image_dirty || self.base_texture.is_none() {
            let image = &self.state.image.buffer;
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            );

            self.base_texture = Some(ctx.load_texture(
                "base_image",
                color_image,
                TextureOptions::NEAREST, // Pixel art friendly
            ));
            self.image_dirty = false;
        }

        // Update layer texture from tool
        if let Some((layer, _x, _y)) = self.state.active_tool.get_temp_layer() {
            // Optimization: Only upload if changed?
            // For now, upload every frame if tool has layer
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [layer.width() as usize, layer.height() as usize],
                layer.as_raw(),
            );
            self.layer_texture =
                Some(ctx.load_texture("temp_layer", color_image, TextureOptions::NEAREST));
        } else {
            self.layer_texture = None;
        }
    }

    fn render_canvas(&mut self, ui: &mut Ui) {
        let canvas_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(canvas_size, Sense::drag());

        let image_size = Vec2::new(
            self.state.image.width() as f32,
            self.state.image.height() as f32,
        ) * self.zoom;

        // Center image if smaller than canvas, else respect pan
        let screen_center = response.rect.center();
        let image_rect = Rect::from_center_size(screen_center + self.pan, image_size);

        // Draw Base
        if let Some(texture) = &self.base_texture {
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        // Draw Layer
        if let Some(texture) = &self.layer_texture {
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        // Draw Canvas Border
        painter.rect_stroke(
            image_rect,
            0.0,
            egui::Stroke::new(1.0, Color32::from_gray(60)),
        );

        // Handle Input
        // Zoom
        if ui.input(|i| i.modifiers.ctrl) {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0 {
                let old_zoom = self.zoom;
                self.zoom *= if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                self.zoom = self.zoom.clamp(0.1, 50.0);

                // Adjust pan to zoom towards pointer?
                // For simplicity, just zoom.
                let _ = old_zoom; // Suppress unused warning
            }
        } else {
            // Pan with Middle Mouse or Space + Drag
            if response.dragged_by(PointerButton::Middle)
                || (ui.input(|i| i.key_down(egui::Key::Space)) && response.dragged())
            {
                self.pan += response.drag_delta();
            }
        }

        // Tool Input
        // We only pass input if we are hovering the image and not panning
        let is_panning = response.dragged_by(PointerButton::Middle)
            || ui.input(|i| i.key_down(egui::Key::Space));

        if !is_panning {
            let pointer_pos = response.interact_pointer_pos();
            let hover_pos_in_image = pointer_pos.map(|pos| {
                // Transform screen pos to image pixel coords
                let relative = pos - image_rect.min;
                let x = (relative.x / self.zoom) as i32;
                let y = (relative.y / self.zoom) as i32;
                Pos2::new(x as f32, y as f32)
            });

            // Filter out of bounds?
            // Tool might handle OOB, but usually we only paint inside.

            let input = ToolInput {
                pos: hover_pos_in_image,
                is_pressed: response.dragged_by(PointerButton::Primary)
                    || response.drag_started_by(PointerButton::Primary), // is_down
                is_released: response.drag_released_by(PointerButton::Primary),
            };

            // Call Tool Update
            let command = self.state.active_tool.update(
                &mut self.state.image,
                &input,
                self.state.primary_color,
            );

            if let Some(cmd) = command {
                // Execute immediately?
                // Wait, the tool already modified the image?
                // Our BrushTool modifies the Layer.
                // It returns a Command when it Commits (merges Layer to Image).
                // But the Command.undo needs to restore the state.
                // The Command returned by BrushTool contains the "Undo" logic (copy old patch).
                // BUT `BrushTool::update` does NOT execute the merge.
                // Wait, `BrushTool` merge logic:
                // "Extract old patch -> Blend -> Extract new patch -> Return Command".
                // Yes, `BrushTool` DOES the merge inside `update`.
                // So the Image is ALREADY modified.
                // We just need to push the command to the stack.
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }

            // Draw Cursor
            if let Some(pos) = pointer_pos {
                if image_rect.contains(pos) {
                    self.state.active_tool.draw_cursor(ui, &painter, pos);
                }
            }
        }
    }
}

impl eframe::App for ArsApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.update_textures(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("ArsPaint");
                ui.separator();

                if ui.button("Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "bmp"])
                        .pick_file()
                    {
                        match crate::image_store::ImageStore::from_file(&path) {
                            Ok(store) => {
                                self.state.image = store;
                                self.state.command_stack = crate::commands::CommandStack::new();
                                self.base_texture = None;
                                self.image_dirty = true;
                            }
                            Err(e) => log::error!("Failed to open: {}", e),
                        }
                    }
                }
                if ui.button("Save").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "bmp"])
                        .save_file()
                    {
                        if let Err(e) = self.state.image.save(&path) {
                            log::error!("Failed to save: {}", e);
                        }
                    }
                }

                ui.separator();

                if ui.button("Undo").clicked() {
                    self.state.command_stack.undo(&mut self.state.image);
                    self.image_dirty = true;
                }
                if ui.button("Redo").clicked() {
                    self.state.command_stack.redo(&mut self.state.image);
                    self.image_dirty = true;
                }

                ui.separator();
                ui.label("Tool:");

                if ui.button("Brush").clicked() {
                    self.state.active_tool = Box::new(crate::tools::BrushTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if ui.button("Eraser").clicked() {
                    self.state.active_tool = Box::new(crate::tools::EraserTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if ui.button("Line").clicked() {
                    self.state.active_tool = Box::new(crate::tools::LineTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if ui.button("Rect").clicked() {
                    self.state.active_tool = Box::new(crate::tools::RectangleTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if ui.button("Ellipse").clicked() {
                    self.state.active_tool = Box::new(crate::tools::EllipseTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }

                ui.label(format!("Active: {}", self.state.active_tool.name()));

                self.state.active_tool.configure(ui);

                ui.separator();
                ui.label("Color:");
                let mut color = [
                    self.state.primary_color[0],
                    self.state.primary_color[1],
                    self.state.primary_color[2],
                ];
                if ui.color_edit_button_srgb(&mut color).changed() {
                    self.state.primary_color = Rgba([color[0], color[1], color[2], 255]);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
    }
}
