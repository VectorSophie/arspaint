use crate::layers::Layer;
use crate::state::AppState;
use crate::tools::ToolInput;
use eframe::egui::{
    self, Color32, Context, PointerButton, Pos2, Rect, Sense, TextureOptions, Ui, Vec2,
};
use eframe::Frame;
use image::Rgba;

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
        // Update base texture from composite if dirty
        // Note: image_store.get_composite() handles dirty checking internally for the buffer
        let composite = self.state.image.get_composite();

        // We still need to upload to GPU if changed
        // Use a simple checksum or just the image_dirty flag from AppState?
        // AppState doesn't track dirty, ImageStore does.
        // But ImageStore.get_composite() returns ref.
        // We need a way to know if we need to call load_texture.
        // Let's rely on self.image_dirty which we set when tools commit.

        if self.image_dirty || self.base_texture.is_none() {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [composite.width() as usize, composite.height() as usize],
                composite.as_raw(),
            );

            self.base_texture =
                Some(ctx.load_texture("base_image", color_image, TextureOptions::NEAREST));
            self.image_dirty = false;
        }

        // Update layer texture from tool
        if let Some((layer, _x, _y)) = self.state.active_tool.get_temp_layer() {
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

    fn render_layers_panel(&mut self, ui: &mut Ui) {
        ui.heading("Layers");
        ui.separator();

        if ui.button("Add Layer").clicked() {
            let idx = self.state.image.layers.len() + 1;
            let layer = Layer::new_raster(
                self.state.image.width(),
                self.state.image.height(),
                format!("Layer {}", idx),
            );
            self.state.image.add_layer(layer);
            self.image_dirty = true;
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Iterate in reverse to show Top layer at Top of list
            let indices: Vec<usize> = (0..self.state.image.layers.len()).rev().collect();

            for idx in indices {
                let is_active = idx == self.state.image.active_layer;

                ui.horizontal(|ui| {
                    // Visibility toggle
                    let mut visible = self.state.image.layers[idx].visible;
                    if ui.checkbox(&mut visible, "👁").changed() {
                        self.state.image.layers[idx].visible = visible;
                        self.state.image.mark_dirty();
                        self.image_dirty = true;
                    }

                    let mut alpha_locked = self.state.image.layers[idx].alpha_locked;
                    if ui
                        .checkbox(&mut alpha_locked, "🔒")
                        .on_hover_text("Lock Transparent Pixels")
                        .changed()
                    {
                        self.state.image.layers[idx].alpha_locked = alpha_locked;
                    }

                    let mut clipped = self.state.image.layers[idx].clipped;
                    if ui
                        .checkbox(&mut clipped, "🖇")
                        .on_hover_text("Clip to Layer Below")
                        .changed()
                    {
                        self.state.image.layers[idx].clipped = clipped;
                        self.state.image.mark_dirty();
                    }

                    // Selection
                    let name = self.state.image.layers[idx].name.clone();
                    let response = ui.selectable_label(is_active, &name);
                    if response.clicked() {
                        self.state.image.active_layer = idx;
                    }
                });

                // Layer properties if active
                if is_active {
                    let layer = &mut self.state.image.layers[idx];
                    let mut opacity = layer.opacity;
                    let mut blend = layer.blend;

                    ui.indent(format!("props_{}", idx), |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Opacity");
                            if ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0)).changed() {
                                // Will update after closure
                            }
                        });

                        egui::ComboBox::from_label("Blend")
                            .selected_text(format!("{:?}", blend))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut blend,
                                    crate::layers::BlendMode::Normal,
                                    "Normal",
                                );
                                ui.selectable_value(
                                    &mut blend,
                                    crate::layers::BlendMode::Multiply,
                                    "Multiply",
                                );
                                ui.selectable_value(
                                    &mut blend,
                                    crate::layers::BlendMode::Add,
                                    "Add",
                                );
                                ui.selectable_value(
                                    &mut blend,
                                    crate::layers::BlendMode::Screen,
                                    "Screen",
                                );
                            });
                    });

                    // Apply changes
                    let layer_mut = &mut self.state.image.layers[idx];
                    if layer_mut.opacity != opacity || layer_mut.blend != blend {
                        layer_mut.opacity = opacity;
                        layer_mut.blend = blend;
                        self.state.image.mark_dirty();
                        self.image_dirty = true;
                    }
                }
            }
        });
    }

    fn render_canvas(&mut self, ui: &mut Ui) {
        let canvas_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(canvas_size, Sense::drag());

        let image_size = Vec2::new(
            self.state.image.width() as f32,
            self.state.image.height() as f32,
        ) * self.zoom;

        let screen_center = response.rect.center();
        let image_rect = Rect::from_center_size(screen_center + self.pan, image_size);

        let checker_size = 16.0 * self.zoom;
        let mut checker_painter = painter.with_clip_rect(image_rect);
        checker_painter.rect_filled(image_rect, 0.0, Color32::from_gray(200));

        let rows = (image_rect.height() / checker_size).ceil() as i32;
        let cols = (image_rect.width() / checker_size).ceil() as i32;

        for r in 0..rows {
            for c in 0..cols {
                if (r + c) % 2 == 1 {
                    let rect = Rect::from_min_size(
                        image_rect.min
                            + Vec2::new(c as f32 * checker_size, r as f32 * checker_size),
                        Vec2::splat(checker_size),
                    );
                    checker_painter.rect_filled(
                        rect.intersect(image_rect),
                        0.0,
                        Color32::from_gray(180),
                    );
                }
            }
        }

        // Draw Composite Base
        if let Some(texture) = &self.base_texture {
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        // Draw Temp Tool Layer (e.g. brush stroke in progress)
        if let Some(texture) = &self.layer_texture {
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        // Canvas Border
        painter.rect_stroke(
            image_rect,
            0.0,
            egui::Stroke::new(1.0, Color32::from_gray(60)),
        );

        // Input Handling
        if ui.input(|i| i.modifiers.ctrl) {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0 {
                let old_zoom = self.zoom;
                self.zoom *= if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                self.zoom = self.zoom.clamp(0.1, 50.0);
                let _ = old_zoom;
            }
        } else {
            if response.dragged_by(PointerButton::Middle)
                || (ui.input(|i| i.key_down(egui::Key::Space)) && response.dragged())
            {
                self.pan += response.drag_delta();
            }
        }

        let is_panning = response.dragged_by(PointerButton::Middle)
            || ui.input(|i| i.key_down(egui::Key::Space));

        if !is_panning {
            let pointer_pos = response.interact_pointer_pos();
            let hover_pos_in_image = pointer_pos.map(|pos| {
                let relative = pos - image_rect.min;
                let x = (relative.x / self.zoom) as i32;
                let y = (relative.y / self.zoom) as i32;
                Pos2::new(x as f32, y as f32)
            });

            let input = ToolInput {
                pos: hover_pos_in_image,
                is_pressed: response.dragged_by(PointerButton::Primary)
                    || response.drag_started_by(PointerButton::Primary),
                is_released: response.drag_stopped_by(PointerButton::Primary),
            };

            let command = self.state.active_tool.update(
                &mut self.state.image,
                &self.state.tool_settings,
                &input,
                self.state.primary_color,
            );

            if let Some(cmd) = command {
                self.state.command_stack.push(cmd);
                self.image_dirty = true;
            }

            if let Some(pos) = pointer_pos {
                if image_rect.contains(pos) {
                    self.state.active_tool.draw_cursor(
                        ui,
                        &painter,
                        &self.state.tool_settings,
                        pos,
                    );
                }
            }
        }
    }
}

impl eframe::App for ArsApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.update_textures(ctx);

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .show(ctx, |ui| {
                self.render_layers_panel(ui);
            });

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

                self.state
                    .active_tool
                    .configure(ui, &mut self.state.tool_settings);

                ui.separator();
                ui.label("Color:");

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let mut color = [
                            self.state.primary_color[0],
                            self.state.primary_color[1],
                            self.state.primary_color[2],
                        ];
                        if ui.color_edit_button_srgb(&mut color).changed() {
                            self.state.primary_color = Rgba([color[0], color[1], color[2], 255]);
                        }
                        ui.label("Primary");
                    });

                    ui.horizontal_wrapped(|ui| {
                        for i in 0..self.state.palette.len() {
                            let p_color = self.state.palette[i];
                            let c32 = Color32::from_rgba_unmultiplied(
                                p_color[0], p_color[1], p_color[2], p_color[3],
                            );

                            let (rect, response) =
                                ui.allocate_at_least(Vec2::splat(18.0), Sense::click());
                            ui.painter().rect_filled(rect, 2.0, c32);
                            if response.clicked() {
                                self.state.primary_color = p_color;
                            }
                            if response.secondary_clicked() {
                                self.state.palette[i] = self.state.primary_color;
                            }
                        }
                        if ui
                            .button("+")
                            .on_hover_text("Add current color to palette")
                            .clicked()
                        {
                            self.state.palette.push(self.state.primary_color);
                        }
                    });
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
    }
}
