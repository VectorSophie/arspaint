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
    selection_texture: Option<egui::TextureHandle>,
    zoom: f32,
    pan: Vec2,
    image_dirty: bool,
    show_shortcuts: bool,
    remapping: Option<String>,
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
            selection_texture: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            image_dirty: true,
            show_shortcuts: false,
            remapping: None,
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

        if let Some(mask) = &self.state.image.selection {
            let mut rgba = Vec::with_capacity(mask.width() as usize * mask.height() as usize * 4);
            for p in mask.pixels() {
                rgba.push(0);
                rgba.push(100);
                rgba.push(255);
                rgba.push(if p[0] > 0 { 50 } else { 0 });
            }
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [mask.width() as usize, mask.height() as usize],
                &rgba,
            );
            self.selection_texture =
                Some(ctx.load_texture("selection_mask", color_image, TextureOptions::NEAREST));
        } else {
            self.selection_texture = None;
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

    fn render_shortcuts_popup(&mut self, ctx: &Context) {
        let mut open = self.show_shortcuts;
        egui::Window::new("Key Mappings")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let bindings = &mut self.state.keybindings;

                    fn shortcut_row(
                        ui: &mut Ui,
                        label: &str,
                        shortcut: &mut crate::state::Shortcut,
                        remapping: &mut Option<String>,
                    ) {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            let text = if remapping.as_deref() == Some(label) {
                                "Press any key...".to_string()
                            } else {
                                shortcut.format()
                            };
                            if ui.button(text).clicked() {
                                *remapping = Some(label.to_string());
                            }
                        });
                    }

                    shortcut_row(ui, "Undo", &mut bindings.undo, &mut self.remapping);
                    shortcut_row(ui, "Redo", &mut bindings.redo, &mut self.remapping);
                    shortcut_row(ui, "Brush", &mut bindings.brush, &mut self.remapping);
                    shortcut_row(ui, "Eraser", &mut bindings.eraser, &mut self.remapping);
                    shortcut_row(ui, "Line", &mut bindings.line, &mut self.remapping);
                    shortcut_row(ui, "Rectangle", &mut bindings.rect, &mut self.remapping);
                    shortcut_row(ui, "Ellipse", &mut bindings.ellipse, &mut self.remapping);
                    shortcut_row(ui, "Select", &mut bindings.select, &mut self.remapping);
                    shortcut_row(ui, "Deselect", &mut bindings.deselect, &mut self.remapping);
                    shortcut_row(
                        ui,
                        "Transform",
                        &mut bindings.transform,
                        &mut self.remapping,
                    );

                    ui.horizontal(|ui| {
                        ui.label("Pan (Modifier):");
                        let text = if self.remapping.as_deref() == Some("Pan") {
                            "Press any key...".to_string()
                        } else {
                            format!("{:?}", bindings.pan)
                        };
                        if ui.button(text).clicked() {
                            self.remapping = Some("Pan".to_string());
                        }
                    });
                });

                if let Some(label) = &self.remapping {
                    let input = ui.input(|i| i.clone());
                    if let Some(key) = input.keys_down.iter().next() {
                        let bindings = &mut self.state.keybindings;
                        match label.as_str() {
                            "Undo" => {
                                bindings.undo = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Redo" => {
                                bindings.redo = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Brush" => {
                                bindings.brush = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Eraser" => {
                                bindings.eraser = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Line" => {
                                bindings.line = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Rectangle" => {
                                bindings.rect = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Ellipse" => {
                                bindings.ellipse = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Select" => {
                                bindings.select = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Deselect" => {
                                bindings.deselect = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Transform" => {
                                bindings.transform = crate::state::Shortcut::new(*key)
                                    .ctrl(input.modifiers.ctrl)
                                    .shift(input.modifiers.shift)
                                    .alt(input.modifiers.alt)
                            }
                            "Pan" => bindings.pan = *key,
                            _ => {}
                        }
                        self.remapping = None;
                    }
                }
            });
        self.show_shortcuts = open;
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

        if let Some(texture) = &self.selection_texture {
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

        let handle_size = 6.0;
        let right_handle =
            Rect::from_center_size(image_rect.right_center(), Vec2::splat(handle_size));
        let bottom_handle =
            Rect::from_center_size(image_rect.center_bottom(), Vec2::splat(handle_size));
        let corner_handle =
            Rect::from_center_size(image_rect.right_bottom(), Vec2::splat(handle_size));

        let mut draw_handle = |rect: Rect, id_str: &str, cursor: egui::CursorIcon| {
            let id = ui.make_persistent_id(id_str);
            let response = ui.interact(rect, id, Sense::drag());
            let color = if response.hovered() || response.dragged() {
                Color32::WHITE
            } else {
                Color32::from_gray(150)
            };
            painter.rect_filled(rect, 0.0, color);
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::BLACK));
            if response.hovered() || response.dragged() {
                ui.output_mut(|o| o.cursor_icon = cursor);
            }
            response
        };

        let h_right = draw_handle(right_handle, "h_right", egui::CursorIcon::ResizeHorizontal);
        let h_bottom = draw_handle(bottom_handle, "h_bottom", egui::CursorIcon::ResizeVertical);
        let h_corner = draw_handle(corner_handle, "h_corner", egui::CursorIcon::ResizeNwSe);

        if h_right.dragged() {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let new_w = ((mouse_pos.x - image_rect.left()) / self.zoom).max(1.0) as u32;
                self.state.image.resize(new_w, self.state.image.height());
                self.image_dirty = true;
            }
        }
        if h_bottom.dragged() {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let new_h = ((mouse_pos.y - image_rect.top()) / self.zoom).max(1.0) as u32;
                self.state.image.resize(self.state.image.width(), new_h);
                self.image_dirty = true;
            }
        }
        if h_corner.dragged() {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let new_w = ((mouse_pos.x - image_rect.left()) / self.zoom).max(1.0) as u32;
                let new_h = ((mouse_pos.y - image_rect.top()) / self.zoom).max(1.0) as u32;
                self.state.image.resize(new_w, new_h);
                self.image_dirty = true;
            }
        }

        // Input Handling
        let bindings = &self.state.keybindings;
        let ctrl = ui.input(|i| i.modifiers.ctrl);

        if ctrl {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0 {
                let old_zoom = self.zoom;
                self.zoom *= if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                self.zoom = self.zoom.clamp(0.1, 50.0);
                let _ = old_zoom;
            }
        } else {
            if response.dragged_by(PointerButton::Middle)
                || (ui.input(|i| i.key_down(bindings.pan)) && response.dragged())
            {
                self.pan += response.drag_delta();
            }
        }

        let is_panning =
            response.dragged_by(PointerButton::Middle) || ui.input(|i| i.key_down(bindings.pan));

        if !is_panning {
            ui.input(|i| {
                if bindings.undo.matches(i) {
                    self.state.command_stack.undo(&mut self.state.image);
                    self.image_dirty = true;
                }
                if bindings.redo.matches(i) {
                    self.state.command_stack.redo(&mut self.state.image);
                    self.image_dirty = true;
                }
                if bindings.brush.matches(i) {
                    self.state.active_tool = Box::new(crate::tools::BrushTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if bindings.eraser.matches(i) {
                    self.state.active_tool = Box::new(crate::tools::EraserTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if bindings.line.matches(i) {
                    self.state.active_tool = Box::new(crate::tools::LineTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if bindings.rect.matches(i) {
                    self.state.active_tool = Box::new(crate::tools::RectangleTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if bindings.ellipse.matches(i) {
                    self.state.active_tool = Box::new(crate::tools::EllipseTool::new(
                        self.state.image.width(),
                        self.state.image.height(),
                    ));
                }
                if bindings.select.matches(i) {
                    self.state.active_tool =
                        Box::new(crate::tools::selection::RectSelectionTool::new());
                }
                if bindings.deselect.matches(i) {
                    self.state.image.selection = None;
                }
                if bindings.transform.matches(i) {
                    self.state.active_tool = Box::new(crate::tools::TransformTool::new());
                }
            });

            let pointer_pos = response.interact_pointer_pos();
            let hover_pos_in_image = pointer_pos.map(|pos| {
                let relative = pos - image_rect.min;
                let x = (relative.x / self.zoom) as i32;
                let y = (relative.y / self.zoom) as i32;
                Pos2::new(x as f32, y as f32)
            });

            let is_right_click = response.dragged_by(PointerButton::Secondary)
                || response.drag_started_by(PointerButton::Secondary);

            let input = ToolInput {
                pos: hover_pos_in_image,
                is_pressed: response.dragged_by(PointerButton::Primary)
                    || response.drag_started_by(PointerButton::Primary)
                    || is_right_click,
                is_released: response.drag_stopped_by(PointerButton::Primary)
                    || response.drag_stopped_by(PointerButton::Secondary),
            };

            let draw_color = if is_right_click {
                self.state.secondary_color
            } else {
                self.state.primary_color
            };

            let command = self.state.active_tool.update(
                &mut self.state.image,
                &self.state.tool_settings,
                &input,
                draw_color,
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
        self.render_shortcuts_popup(ctx);

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .show(ctx, |ui| {
                self.render_layers_panel(ui);
            });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("ArsPaint");
                ui.separator();

                if ui.button("Shortcuts").clicked() {
                    self.show_shortcuts = true;
                }

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
                if ui.button("Select").clicked() {
                    self.state.active_tool = Box::new(crate::tools::RectSelectionTool::new());
                }
                if ui.button("Lasso").clicked() {
                    self.state.active_tool = Box::new(crate::tools::LassoSelectionTool::new());
                }
                if ui.button("Transform").clicked() {
                    self.state.active_tool = Box::new(crate::tools::TransformTool::new());
                }

                ui.label(format!("Active: {}", self.state.active_tool.name()));

                self.state
                    .active_tool
                    .configure(ui, &mut self.state.tool_settings);

                ui.separator();
                ui.label("Color:");

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let mut p_color = [
                            self.state.primary_color[0],
                            self.state.primary_color[1],
                            self.state.primary_color[2],
                        ];
                        if ui.color_edit_button_srgb(&mut p_color).changed() {
                            self.state.primary_color =
                                Rgba([p_color[0], p_color[1], p_color[2], 255]);
                        }
                        ui.label("Primary");

                        ui.separator();

                        let mut s_color = [
                            self.state.secondary_color[0],
                            self.state.secondary_color[1],
                            self.state.secondary_color[2],
                        ];
                        if ui.color_edit_button_srgb(&mut s_color).changed() {
                            self.state.secondary_color =
                                Rgba([s_color[0], s_color[1], s_color[2], 255]);
                        }
                        ui.label("Secondary");
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
                                self.state.secondary_color = p_color;
                            }
                            if response.middle_clicked() {
                                self.state.palette[i] = self.state.primary_color;
                            }
                        }
                        if ui
                            .button("+")
                            .on_hover_text("Add current primary to palette")
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
