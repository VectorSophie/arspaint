use crate::image_store::ImageStore;
use crate::layers::Layer;
use crate::state::{AppState, FillMode, FloatingSelection, StrokeSize, PALETTE};
use crate::tools::{
    AirbrushTool, BrushTool, CurveTool, EllipseTool, EraserTool, EyedropperTool, FillTool,
    LassoSelectionTool, LineTool, PencilTool, RectSelectionTool, RectangleTool, ShapeKind,
    ShapeTool, ToolInput, TransformTool,
};
use eframe::egui::{
    self, Color32, Context, PointerButton, Pos2, Rect, Sense, TextureOptions, Ui, Vec2,
};
use eframe::Frame;
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

#[derive(PartialEq, Clone, Copy)]
enum RibbonTab { Home, View }

#[derive(PartialEq, Clone, Copy)]
enum SelectDropdown { Rect, Lasso, All }

pub struct ArsApp {
    state: AppState,
    base_texture: Option<egui::TextureHandle>,
    layer_texture: Option<egui::TextureHandle>,
    selection_texture: Option<egui::TextureHandle>,
    zoom: f32,
    pan: Vec2,
    image_dirty: bool,
    ribbon_tab: RibbonTab,
    show_grid: bool,
    show_status_bar: bool,
    cursor_pos: Option<Pos2>,
    // dialogs
    show_resize_dialog: bool,
    resize_w: u32,
    resize_h: u32,
    resize_pct: bool,
    resize_lock_aspect: bool,
    show_new_dialog: bool,
    new_w: u32,
    new_h: u32,
    show_rotate_menu: bool,
    show_select_menu: bool,
    // eyedropper alt-mode
    alt_eyedropper_active: bool,
    pre_alt_tool_name: String,
}

impl ArsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = Color32::from_rgb(26, 27, 38);
        visuals.panel_fill = Color32::from_rgb(26, 27, 38);
        visuals.extreme_bg_color = Color32::from_rgb(22, 22, 30);
        cc.egui_ctx.set_visuals(visuals);

        let state = AppState::new(800, 600);
        let (w, h) = (state.image.width(), state.image.height());
        Self {
            state,
            base_texture: None,
            layer_texture: None,
            selection_texture: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            image_dirty: true,
            ribbon_tab: RibbonTab::Home,
            show_grid: false,
            show_status_bar: true,
            cursor_pos: None,
            show_resize_dialog: false,
            resize_w: w,
            resize_h: h,
            resize_pct: false,
            resize_lock_aspect: true,
            show_new_dialog: false,
            new_w: 800,
            new_h: 600,
            show_rotate_menu: false,
            show_select_menu: false,
            alt_eyedropper_active: false,
            pre_alt_tool_name: String::new(),
        }
    }

    fn update_textures(&mut self, ctx: &Context) {
        let composite = self.state.image.get_composite();
        if self.image_dirty || self.base_texture.is_none() {
            let ci = egui::ColorImage::from_rgba_unmultiplied(
                [composite.width() as usize, composite.height() as usize],
                composite.as_raw(),
            );
            self.base_texture = Some(ctx.load_texture("base_image", ci, TextureOptions::NEAREST));
            self.image_dirty = false;
        }

        if let Some((layer, _x, _y)) = self.state.active_tool.get_temp_layer() {
            let ci = egui::ColorImage::from_rgba_unmultiplied(
                [layer.width() as usize, layer.height() as usize],
                layer.as_raw(),
            );
            self.layer_texture = Some(ctx.load_texture("temp_layer", ci, TextureOptions::NEAREST));
        } else {
            self.layer_texture = None;
        }

        // floating selection overlay
        if let Some(fs) = &self.state.floating_selection {
            let ci = egui::ColorImage::from_rgba_unmultiplied(
                [fs.image.width() as usize, fs.image.height() as usize],
                fs.image.as_raw(),
            );
            // reuse layer_texture slot for floating selection when no tool temp layer
            if self.layer_texture.is_none() {
                self.layer_texture = Some(ctx.load_texture("float_sel", ci, TextureOptions::NEAREST));
            }
        }

        if let Some(mask) = &self.state.image.selection {
            let mut rgba = Vec::with_capacity(mask.width() as usize * mask.height() as usize * 4);
            for p in mask.pixels() {
                rgba.extend_from_slice(&[0, 100, 255, if p[0] > 0 { 50 } else { 0 }]);
            }
            let ci = egui::ColorImage::from_rgba_unmultiplied(
                [mask.width() as usize, mask.height() as usize], &rgba,
            );
            self.selection_texture = Some(ctx.load_texture("selection_mask", ci, TextureOptions::NEAREST));
        } else {
            self.selection_texture = None;
        }
    }

    // ── Ribbon ────────────────────────────────────────────────────────────────

    fn render_ribbon(&mut self, ui: &mut Ui) {
        // Tab bar
        ui.horizontal(|ui| {
            let home_color = if self.ribbon_tab == RibbonTab::Home { Color32::from_rgb(122, 162, 247) } else { Color32::GRAY };
            let view_color = if self.ribbon_tab == RibbonTab::View  { Color32::from_rgb(122, 162, 247) } else { Color32::GRAY };
            if ui.add(egui::Button::new(egui::RichText::new("Home").color(home_color)).frame(false)).clicked() {
                self.ribbon_tab = RibbonTab::Home;
            }
            if ui.add(egui::Button::new(egui::RichText::new("View").color(view_color)).frame(false)).clicked() {
                self.ribbon_tab = RibbonTab::View;
            }
        });
        ui.separator();

        match self.ribbon_tab {
            RibbonTab::Home => self.render_home_tab(ui),
            RibbonTab::View => self.render_view_tab(ui),
        }
    }

    fn render_home_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // ── Clipboard ──
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Clipboard").small().color(Color32::GRAY));
                ui.horizontal(|ui| {
                    if ui.button("Paste").clicked() { self.do_paste(); }
                });
                ui.horizontal(|ui| {
                    if ui.button("Cut").clicked() { self.do_cut(); }
                    if ui.button("Copy").clicked() { self.do_copy(); }
                });
            });
            ui.separator();

            // ── Image ──
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Image").small().color(Color32::GRAY));
                ui.horizontal(|ui| {
                    // Select dropdown
                    if ui.button("Select ▾").clicked() { self.show_select_menu = !self.show_select_menu; }
                    if ui.button("Crop").clicked() { self.do_crop(); }
                });
                ui.horizontal(|ui| {
                    if ui.button("Resize").clicked() {
                        self.resize_w = self.state.image.width();
                        self.resize_h = self.state.image.height();
                        self.show_resize_dialog = true;
                    }
                    if ui.button("Rotate ▾").clicked() { self.show_rotate_menu = !self.show_rotate_menu; }
                });
            });
            ui.separator();

            // ── Tools ──
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Tools").small().color(Color32::GRAY));
                let active = self.state.active_tool.name().to_string();
                let is_pencil  = active == "Pencil";
                let is_fill    = active == "Fill";
                let is_pick    = active == "Color Picker";
                let is_eraser  = active == "Eraser";
                let clicked_pencil = ui.horizontal(|ui| {
                    let a = ui.selectable_label(is_pencil, "✏ Pencil").clicked();
                    let b = ui.selectable_label(is_fill,   "⬛ Fill").clicked();
                    let c = ui.selectable_label(is_pick,   "💧 Pick").clicked();
                    (a, b, c)
                }).inner;
                let clicked_row2 = ui.horizontal(|ui| {
                    let a = ui.selectable_label(false,     "A Text").clicked();
                    let b = ui.selectable_label(is_eraser, "⬜ Eraser").clicked();
                    let c = ui.selectable_label(false,     "🔍 Zoom").clicked();
                    (a, b, c)
                }).inner;
                if clicked_pencil.0 { self.set_tool_pencil(); }
                if clicked_pencil.1 { self.state.active_tool = Box::new(FillTool::new()); }
                if clicked_pencil.2 { self.state.active_tool = Box::new(EyedropperTool::new()); }
                if clicked_row2.1   { self.set_tool_eraser(); }
                // Stroke size
                ui.horizontal(|ui| {
                    for (size, label) in [(StrokeSize::Thin,"—"),(StrokeSize::Medium,"─"),(StrokeSize::Thick,"━"),(StrokeSize::ExtraThick,"█")] {
                        if ui.selectable_label(self.state.tool_settings.stroke_size == size, label).clicked() {
                            self.state.tool_settings.stroke_size = size;
                            self.state.tool_settings.line_width = size.px();
                            self.state.tool_settings.eraser_size = size.px() * 2.0;
                            self.state.tool_settings.brush_size = size.px() * 2.0;
                        }
                    }
                });
            });
            ui.separator();

            // ── Shapes ──
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Shapes").small().color(Color32::GRAY));
                let active = self.state.active_tool.name().to_string();
                let img_w = self.state.image.width();
                let img_h = self.state.image.height();

                // Line tools row 1
                let (cl, cc, ct, cd) = ui.horizontal(|ui| (
                    ui.selectable_label(active == "Line",  "╱ Line").clicked(),
                    ui.selectable_label(active == "Curve", "〰 Curve").clicked(),
                    ui.selectable_label(active == "Shape", "△ Tri").clicked(),
                    ui.selectable_label(active == "Shape", "◇ Dia").clicked(),
                )).inner;
                if cl { self.state.active_tool = Box::new(LineTool::new(img_w, img_h)); }
                if cc { self.state.active_tool = Box::new(CurveTool::new(img_w, img_h)); }
                if ct { self.state.active_tool = Box::new(ShapeTool::new(ShapeKind::Triangle, img_w, img_h)); }
                if cd { self.state.active_tool = Box::new(ShapeTool::new(ShapeKind::Diamond, img_w, img_h)); }

                let (cr, ce) = ui.horizontal(|ui| (
                    ui.selectable_label(active == "Rectangle", "▭ Rect").clicked(),
                    ui.selectable_label(active == "Ellipse",   "⬭ Oval").clicked(),
                )).inner;
                if cr { self.state.active_tool = Box::new(RectangleTool::new(img_w, img_h)); }
                if ce { self.state.active_tool = Box::new(EllipseTool::new(img_w, img_h)); }

                let shape_clicks: Vec<(ShapeKind, bool)> = ui.horizontal(|ui| {
                    [
                        (ShapeKind::RightTriangle,"◺"),(ShapeKind::Pentagon,"⬠"),
                        (ShapeKind::Hexagon,"⬡"),(ShapeKind::ArrowRight,"➡"),
                        (ShapeKind::ArrowLeft,"⬅"),(ShapeKind::ArrowUp,"⬆"),
                        (ShapeKind::ArrowDown,"⬇"),(ShapeKind::Arrow4Way,"✛"),
                        (ShapeKind::ArrowLeftRight,"↔"),(ShapeKind::ArrowUpDown,"↕"),
                        (ShapeKind::Star4,"✦"),(ShapeKind::Star6,"✶"),
                        (ShapeKind::CalloutRect,"💬"),(ShapeKind::CalloutOval,"🗨"),
                        (ShapeKind::CalloutCloud,"☁"),(ShapeKind::Heart,"♥"),
                        (ShapeKind::Polygon,"⬡̣"),
                    ].iter().map(|(kind, label)| {
                        (*kind, ui.selectable_label(false, *label).on_hover_text(format!("{:?}", kind)).clicked())
                    }).collect()
                }).inner;
                for (kind, clicked) in shape_clicks {
                    if clicked { self.state.active_tool = Box::new(ShapeTool::new(kind, img_w, img_h)); }
                }
                // Fill mode
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Fill:").small());
                    for (mode, label) in [(FillMode::Outline,"Outline"),(FillMode::Fill,"Fill"),(FillMode::Both,"Both")] {
                        if ui.selectable_label(self.state.tool_settings.shape_fill_mode == mode, label).clicked() {
                            self.state.tool_settings.shape_fill_mode = mode;
                        }
                    }
                });
            });
            ui.separator();

            // ── Colors ──
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Colors").small().color(Color32::GRAY));
                ui.horizontal(|ui| {
                    // Color 1 swatch
                    let c1 = self.state.color1;
                    let c1_32 = Color32::from_rgba_unmultiplied(c1[0], c1[1], c1[2], 255);
                    let (r1, resp1) = ui.allocate_at_least(Vec2::splat(20.0), Sense::click());
                    ui.painter().rect_filled(r1, 3.0, c1_32);
                    ui.painter().rect_stroke(r1, 3.0, egui::Stroke::new(1.0, Color32::WHITE));
                    if resp1.clicked() {
                        // open egui color picker for color1 — handled below via show_color1_picker
                    }

                    // Color 2 swatch
                    let c2 = self.state.color2;
                    let c2_32 = Color32::from_rgba_unmultiplied(c2[0], c2[1], c2[2], 255);
                    let (r2, _resp2) = ui.allocate_at_least(Vec2::splat(20.0), Sense::click());
                    ui.painter().rect_filled(r2, 3.0, c2_32);
                    ui.painter().rect_stroke(r2, 3.0, egui::Stroke::new(1.0, Color32::GRAY));
                });

                // 20-color palette (2 rows of 10)
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

                // Color 1/2 raw editors
                ui.horizontal(|ui| {
                    let mut c1 = [self.state.color1[0], self.state.color1[1], self.state.color1[2]];
                    if ui.color_edit_button_srgb(&mut c1).changed() {
                        self.state.color1 = Rgba([c1[0], c1[1], c1[2], 255]);
                    }
                    ui.label("1");
                    let mut c2 = [self.state.color2[0], self.state.color2[1], self.state.color2[2]];
                    if ui.color_edit_button_srgb(&mut c2).changed() {
                        self.state.color2 = Rgba([c2[0], c2[1], c2[2], 255]);
                    }
                    ui.label("2");
                });
            });
        });
    }

    fn render_view_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Zoom").small().color(Color32::GRAY));
                ui.horizontal(|ui| {
                    if ui.button("＋").clicked() { self.zoom = (self.zoom * 1.25).min(50.0); }
                    if ui.button("－").clicked() { self.zoom = (self.zoom / 1.25).max(0.1); }
                    if ui.button("100%").clicked() { self.zoom = 1.0; }
                    if ui.button("Fit").clicked() {
                        // zoom handled in canvas
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Show or hide").small().color(Color32::GRAY));
                ui.checkbox(&mut self.show_grid, "Gridlines");
                ui.checkbox(&mut self.show_status_bar, "Status bar");
            });
        });
    }

    // ── Dialogs ───────────────────────────────────────────────────────────────

    fn render_dialogs(&mut self, ctx: &Context) {
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

    // ── Canvas ────────────────────────────────────────────────────────────────

    fn render_canvas(&mut self, ui: &mut Ui) {
        let canvas_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(canvas_size, Sense::drag());

        let image_size = Vec2::new(
            self.state.image.width() as f32,
            self.state.image.height() as f32,
        ) * self.zoom;

        let screen_center = response.rect.center();
        let image_rect = Rect::from_center_size(screen_center + self.pan, image_size);

        // Checkerboard background
        let checker_size = 16.0 * self.zoom;
        let checker_painter = painter.with_clip_rect(image_rect);
        checker_painter.rect_filled(image_rect, 0.0, Color32::from_gray(200));
        let rows = (image_rect.height() / checker_size).ceil() as i32;
        let cols = (image_rect.width() / checker_size).ceil() as i32;
        for r in 0..rows {
            for c in 0..cols {
                if (r + c) % 2 == 1 {
                    let rect = Rect::from_min_size(
                        image_rect.min + Vec2::new(c as f32 * checker_size, r as f32 * checker_size),
                        Vec2::splat(checker_size),
                    );
                    checker_painter.rect_filled(rect.intersect(image_rect), 0.0, Color32::from_gray(180));
                }
            }
        }

        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        if let Some(tex) = &self.base_texture {
            painter.image(tex.id(), image_rect, uv, Color32::WHITE);
        }
        if let Some(tex) = &self.layer_texture {
            painter.image(tex.id(), image_rect, uv, Color32::WHITE);
        }
        if let Some(tex) = &self.selection_texture {
            painter.image(tex.id(), image_rect, uv, Color32::WHITE);
        }

        // Grid at high zoom
        if self.show_grid && self.zoom >= 4.0 {
            for gx in 0..=self.state.image.width() {
                let sx = image_rect.min.x + gx as f32 * self.zoom;
                painter.line_segment(
                    [Pos2::new(sx, image_rect.min.y), Pos2::new(sx, image_rect.max.y)],
                    egui::Stroke::new(0.3, Color32::from_rgba_unmultiplied(0,0,0,60)),
                );
            }
            for gy in 0..=self.state.image.height() {
                let sy = image_rect.min.y + gy as f32 * self.zoom;
                painter.line_segment(
                    [Pos2::new(image_rect.min.x, sy), Pos2::new(image_rect.max.x, sy)],
                    egui::Stroke::new(0.3, Color32::from_rgba_unmultiplied(0,0,0,60)),
                );
            }
        }

        // Canvas border
        painter.rect_stroke(image_rect, 0.0, egui::Stroke::new(1.0, Color32::from_gray(60)));

        // Snapshot keybinding values (Copy types) to avoid holding a borrow across closures
        let (kb_undo, kb_redo, kb_sel_all, kb_copy, kb_cut, kb_paste,
             kb_new, kb_open, kb_save, kb_save_as, kb_resize, kb_invert,
             kb_zoom_in, kb_zoom_out, kb_pan,
             kb_pencil, kb_brush, kb_eraser, kb_fill, kb_eye, kb_airbrush, kb_select) = {
            let kb = &self.state.keybindings;
            (kb.undo, kb.redo, kb.select_all, kb.copy, kb.cut, kb.paste,
             kb.new_file, kb.open_file, kb.save, kb.save_as, kb.resize_dialog, kb.invert,
             kb.zoom_in, kb.zoom_out, kb.pan,
             kb.pencil, kb.brush, kb.eraser, kb.fill, kb.eyedropper, kb.airbrush, kb.select)
        };

        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let alt  = ui.input(|i| i.modifiers.alt);

        // Zoom via ctrl+scroll
        if ctrl {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom = (self.zoom * if scroll > 0.0 { 1.1 } else { 0.9 }).clamp(0.1, 50.0);
            }
        }

        // Alt = temporary eyedropper
        if alt && !self.alt_eyedropper_active {
            self.pre_alt_tool_name = self.state.active_tool.name().to_string();
            self.state.active_tool = Box::new(EyedropperTool::new());
            self.alt_eyedropper_active = true;
        } else if !alt && self.alt_eyedropper_active {
            self.alt_eyedropper_active = false;
            self.restore_pre_alt_tool();
        }

        // Pan
        let is_panning = response.dragged_by(PointerButton::Middle)
            || (ui.input(|i| i.key_down(kb_pan)) && response.dragged());
        if is_panning {
            self.pan += response.drag_delta();
        }

        if !is_panning {
            // Keyboard shortcuts — collect needed flags first, apply after
            let (do_undo, do_redo, do_sel_all, do_copy, do_cut, do_paste,
                 do_new, do_open, do_save, do_save_as, do_resize, do_invert,
                 do_zoom_in, do_zoom_out,
                 do_pencil, do_brush, do_eraser, do_fill, do_eye, do_airbrush, do_select,
                 do_escape, do_delete) =
            ui.input(|i| (
                kb_undo.matches(i), kb_redo.matches(i), kb_sel_all.matches(i),
                kb_copy.matches(i), kb_cut.matches(i), kb_paste.matches(i),
                kb_new.matches(i), kb_open.matches(i), kb_save.matches(i), kb_save_as.matches(i),
                kb_resize.matches(i), kb_invert.matches(i),
                kb_zoom_in.matches(i), kb_zoom_out.matches(i),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_pencil)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_brush)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_eraser)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_fill)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_eye)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_airbrush)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_select)),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Delete),
            ));

            let img_w = self.state.image.width();
            let img_h = self.state.image.height();

            if do_undo  { self.state.command_stack.undo(&mut self.state.image); self.image_dirty = true; }
            if do_redo  { self.state.command_stack.redo(&mut self.state.image); self.image_dirty = true; }
            if do_sel_all { self.do_select_all(); }
            if do_copy  { self.do_copy(); }
            if do_cut   { self.do_cut(); }
            if do_paste { self.do_paste(); }
            if do_new   { self.show_new_dialog = true; }
            if do_open  { self.do_open(); }
            if do_save  { self.do_save(); }
            if do_save_as { self.do_save_as(); }
            if do_resize { self.resize_w = img_w; self.resize_h = img_h; self.show_resize_dialog = true; }
            if do_invert { self.state.image.invert_colors(); self.image_dirty = true; }
            if do_zoom_in  { self.zoom = (self.zoom * 1.25).min(50.0); }
            if do_zoom_out { self.zoom = (self.zoom / 1.25).max(0.1); }
            if do_pencil  { self.set_tool_pencil(); }
            if do_brush   { self.state.active_tool = Box::new(BrushTool::new(img_w, img_h)); }
            if do_eraser  { self.set_tool_eraser(); }
            if do_fill    { self.state.active_tool = Box::new(FillTool::new()); }
            if do_eye     { self.state.active_tool = Box::new(EyedropperTool::new()); }
            if do_airbrush{ self.state.active_tool = Box::new(AirbrushTool::new(img_w, img_h)); }
            if do_select  { self.state.active_tool = Box::new(RectSelectionTool::new()); }
            if do_escape  { self.state.image.selection = None; self.state.floating_selection = None; }
            if do_delete  { self.do_delete_selection(); }

            // Tool update
            let pointer_pos = response.interact_pointer_pos();
            let hover_pos_in_image = pointer_pos.map(|pos| {
                let rel = pos - image_rect.min;
                Pos2::new((rel.x / self.zoom) as f32, (rel.y / self.zoom) as f32)
            });

            // Track cursor for status bar
            if let Some(p) = response.hover_pos() {
                let rel = p - image_rect.min;
                self.cursor_pos = Some(Pos2::new(rel.x / self.zoom, rel.y / self.zoom));
            }

            let is_right = response.dragged_by(PointerButton::Secondary)
                || response.drag_started_by(PointerButton::Secondary);

            let input = ToolInput {
                pos: hover_pos_in_image,
                is_pressed: response.dragged_by(PointerButton::Primary)
                    || response.drag_started_by(PointerButton::Primary)
                    || is_right,
                is_released: response.drag_stopped_by(PointerButton::Primary)
                    || response.drag_stopped_by(PointerButton::Secondary),
            };

            let draw_color = if is_right { self.state.color2 } else { self.state.color1 };

            // Handle eyedropper picks
            let cmd = self.state.active_tool.update(
                &mut self.state.image,
                &self.state.tool_settings,
                &input,
                draw_color,
            );

            // Check eyedropper result
            if let Some(eyedrop) = self.state.active_tool.as_any_eyedropper() {
                if let Some((picked, _)) = eyedrop.picked {
                    if is_right { self.state.color2 = picked; } else { self.state.color1 = picked; }
                }
            }

            if let Some(c) = cmd {
                self.state.command_stack.push(c);
                self.image_dirty = true;
            }

            if let Some(pos) = pointer_pos {
                if image_rect.contains(pos) {
                    self.state.active_tool.draw_cursor(ui, &painter, &self.state.tool_settings, pos);
                }
            }
        }
    }

    // ── Status bar ────────────────────────────────────────────────────────────

    fn render_status_bar(&mut self, ui: &mut Ui) {
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

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn shape_kind(&self) -> Option<ShapeKind> {
        // Downcast is not available on Box<dyn Tool> without an as_any pattern;
        // use name-based heuristic since ShapeTool always reports "Shape"
        None // TODO: expose shape kind via Tool trait extension if needed
    }

    fn set_tool_pencil(&mut self) {
        let (w, h) = (self.state.image.width(), self.state.image.height());
        self.state.active_tool = Box::new(PencilTool::new(w, h));
    }

    fn set_tool_eraser(&mut self) {
        let (w, h) = (self.state.image.width(), self.state.image.height());
        self.state.active_tool = Box::new(EraserTool::new(w, h));
    }

    fn restore_pre_alt_tool(&mut self) {
        let (w, h) = (self.state.image.width(), self.state.image.height());
        self.state.active_tool = match self.pre_alt_tool_name.as_str() {
            "Pencil" => Box::new(PencilTool::new(w, h)),
            "Eraser" => Box::new(EraserTool::new(w, h)),
            "Brush"  => Box::new(BrushTool::new(w, h)),
            "Fill"   => Box::new(FillTool::new()),
            "Line"   => Box::new(LineTool::new(w, h)),
            _ => Box::new(PencilTool::new(w, h)),
        };
    }

    fn do_select_all(&mut self) {
        let (w, h) = (self.state.image.width(), self.state.image.height());
        let mut mask = image::GrayImage::new(w, h);
        for p in mask.pixels_mut() { *p = image::Luma([255]); }
        self.state.image.selection = Some(mask);
    }

    fn do_copy(&mut self) {
        let snapshot = self.state.image.get_active_raster_snapshot();
        if let Some(buf) = snapshot {
            if let Some(mask) = &self.state.image.selection {
                // copy only selected region bounding box
                let mut min_x = u32::MAX; let mut min_y = u32::MAX;
                let mut max_x = 0u32; let mut max_y = 0u32;
                for (x, y, p) in mask.enumerate_pixels() {
                    if p[0] > 0 { min_x=min_x.min(x); min_y=min_y.min(y); max_x=max_x.max(x); max_y=max_y.max(y); }
                }
                if max_x >= min_x && max_y >= min_y {
                    let w = max_x - min_x + 1;
                    let h = max_y - min_y + 1;
                    use image::GenericImageView;
                    self.state.clipboard = Some(buf.view(min_x, min_y, w, h).to_image());
                }
            } else {
                self.state.clipboard = Some(buf);
            }
        }
    }

    fn do_cut(&mut self) {
        self.do_copy();
        self.do_delete_selection();
    }

    fn do_paste(&mut self) {
        if let Some(clip) = self.state.clipboard.clone() {
            self.state.floating_selection = Some(FloatingSelection { image: clip, pos: Pos2::new(0.0, 0.0) });
        }
    }

    fn do_delete_selection(&mut self) {
        let color2 = self.state.color2;
        if let Some(mask) = &self.state.image.selection.clone() {
            if let Some(buf) = self.state.image.get_active_raster_buffer_mut() {
                for (x, y, p) in mask.enumerate_pixels() {
                    if p[0] > 0 { buf.put_pixel(x, y, color2); }
                }
            }
            self.state.image.mark_dirty();
            self.image_dirty = true;
        }
    }

    fn do_crop(&mut self) {
        if let Some(mask) = &self.state.image.selection.clone() {
            let mut min_x = u32::MAX; let mut min_y = u32::MAX;
            let mut max_x = 0u32; let mut max_y = 0u32;
            for (x, y, p) in mask.enumerate_pixels() {
                if p[0] > 0 { min_x=min_x.min(x); min_y=min_y.min(y); max_x=max_x.max(x); max_y=max_y.max(y); }
            }
            if max_x >= min_x && max_y >= min_y {
                self.state.image.crop_to(min_x, min_y, max_x-min_x+1, max_y-min_y+1);
                self.image_dirty = true;
            }
        }
    }

    fn do_open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png","jpg","jpeg","bmp","gif","tiff"])
            .pick_file()
        {
            match ImageStore::from_file(&path) {
                Ok(store) => {
                    self.state.image = store;
                    self.state.command_stack = crate::commands::CommandStack::new();
                    self.state.current_save_path = Some(path);
                    self.base_texture = None;
                    self.image_dirty = true;
                }
                Err(e) => log::error!("Failed to open: {}", e),
            }
        }
    }

    fn do_save(&mut self) {
        if let Some(path) = &self.state.current_save_path.clone() {
            if let Err(e) = self.state.image.save(path) { log::error!("Save failed: {}", e); }
        } else {
            self.do_save_as();
        }
    }

    fn do_save_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg","jpeg"])
            .add_filter("BMP", &["bmp"])
            .save_file()
        {
            if let Err(e) = self.state.image.save(&path) {
                log::error!("Save failed: {}", e);
            } else {
                self.state.current_save_path = Some(path);
            }
        }
    }
}

// Trait extension to let ui.rs query the EyedropperTool without full downcast
pub trait AsEyedropper {
    fn as_any_eyedropper(&self) -> Option<&crate::tools::EyedropperTool>;
}

impl<T: crate::tools::Tool + ?Sized> AsEyedropper for Box<T> {
    fn as_any_eyedropper(&self) -> Option<&crate::tools::EyedropperTool> { None }
}

impl eframe::App for ArsApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.update_textures(ctx);
        self.render_dialogs(ctx);

        egui::TopBottomPanel::top("ribbon").show(ctx, |ui| {
            self.render_ribbon(ui);
        });

        if self.show_status_bar {
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                self.render_status_bar(ui);
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
    }
}
