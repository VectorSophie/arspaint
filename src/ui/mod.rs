mod canvas;
mod theme;
use crate::image_store::ImageStore;
use crate::layers::Layer;
use crate::state::{AppState, FillMode, FloatingSelection, StrokeSize, PALETTE};
use crate::tools::{
    AirbrushTool, BrushTool, CurveTool, EllipseTool, EraserTool, EyedropperTool, FillTool,
    LassoSelectionTool, LineTool, PencilTool, RectSelectionTool, RectangleTool, ShapeKind,
    ShapeTool, SmearTool, TransformTool,
};
use eframe::egui::{
    self, Color32, Context, Pos2, Sense, TextureOptions, Ui, Vec2,
};
use eframe::Frame;
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

#[derive(PartialEq, Clone, Copy)]
enum RibbonTab { Home, View }

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
    show_layers_panel: bool,
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
    // Phase D
    show_color_wheel: bool,
}

impl ArsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

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
            show_layers_panel: true,
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
            show_color_wheel: true,
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

        // floating selection — upload as its own texture always
        if let Some(fs) = &self.state.floating_selection {
            let ci = egui::ColorImage::from_rgba_unmultiplied(
                [fs.image.width() as usize, fs.image.height() as usize],
                fs.image.as_raw(),
            );
            self.layer_texture = Some(ctx.load_texture("float_sel", ci, TextureOptions::NEAREST));
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
                let is_smear = active == "Smear";
                let clicked_row2 = ui.horizontal(|ui| {
                    let a = ui.selectable_label(false,     "A Text").clicked();
                    let b = ui.selectable_label(is_eraser, "⬜ Eraser").clicked();
                    let c = ui.selectable_label(false,     "🔍 Zoom").clicked();
                    (a, b, c)
                }).inner;
                let clicked_row3 = ui.horizontal(|ui| {
                    let a = ui.selectable_label(is_smear, "〰 Smear").on_hover_text("Smear/wet mix (W)").clicked();
                    (a,)
                }).inner;
                if clicked_pencil.0 { self.set_tool_pencil(); }
                if clicked_pencil.1 { self.state.active_tool = Box::new(FillTool::new()); }
                if clicked_pencil.2 { self.state.active_tool = Box::new(EyedropperTool::new()); }
                if clicked_row2.1   { self.set_tool_eraser(); }
                if clicked_row3.0   {
                    let (w, h) = (self.state.image.width(), self.state.image.height());
                    self.state.active_tool = Box::new(SmearTool::new(w, h));
                }
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

                let cur_shape = self.shape_kind();
                let shape_clicks: Vec<(ShapeKind, bool)> = ui.horizontal_wrapped(|ui| {
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
                        (*kind, ui.selectable_label(cur_shape == Some(*kind), *label).on_hover_text(format!("{:?}", kind)).clicked())
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

                // Stacked Color 2 behind Color 1 (Paint-style)
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        // Color 2 (background) — bottom swatch
                        let c2 = self.state.color2;
                        let mut c2_arr = [c2[0], c2[1], c2[2]];
                        ui.label(egui::RichText::new("2").small().color(Color32::GRAY));
                        if ui.color_edit_button_srgb(&mut c2_arr).changed() {
                            self.state.color2 = Rgba([c2_arr[0], c2_arr[1], c2_arr[2], 255]);
                        }
                    });
                    ui.vertical(|ui| {
                        // Color 1 (foreground) — top swatch
                        let c1 = self.state.color1;
                        let mut c1_arr = [c1[0], c1[1], c1[2]];
                        ui.label(egui::RichText::new("1").small().color(Color32::GRAY));
                        if ui.color_edit_button_srgb(&mut c1_arr).changed() {
                            self.state.color1 = Rgba([c1_arr[0], c1_arr[1], c1_arr[2], 255]);
                        }
                    });
                    // Swap button
                    if ui.button("⇄").on_hover_text("Swap colors").clicked() {
                        std::mem::swap(&mut self.state.color1, &mut self.state.color2);
                    }
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
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Show or hide").small().color(Color32::GRAY));
                ui.checkbox(&mut self.show_grid, "Gridlines");
                ui.checkbox(&mut self.show_status_bar, "Status bar");
                ui.checkbox(&mut self.show_layers_panel, "Layers");
            });
        });
    }

    fn render_layers_panel(&mut self, ui: &mut Ui) {
        ui.heading("Layers");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("＋").on_hover_text("Add raster layer").clicked() {
                let (w, h) = (self.state.image.width(), self.state.image.height());
                let n = self.state.image.layers.len() + 1;
                self.state.image.add_layer(Layer::new_raster(w, h, format!("Layer {}", n)));
                self.image_dirty = true;
            }
            let can_delete = self.state.image.layers.len() > 1;
            if ui.add_enabled(can_delete, egui::Button::new("✕")).on_hover_text("Delete layer").clicked() {
                let idx = self.state.image.active_layer;
                self.state.image.layers.remove(idx);
                self.state.image.active_layer = idx.saturating_sub(1).min(self.state.image.layers.len() - 1);
                self.state.image.mark_dirty();
                self.image_dirty = true;
            }
            let can_up = self.state.image.active_layer + 1 < self.state.image.layers.len();
            if ui.add_enabled(can_up, egui::Button::new("↑")).on_hover_text("Move up").clicked() {
                let idx = self.state.image.active_layer;
                self.state.image.layers.swap(idx, idx + 1);
                self.state.image.active_layer = idx + 1;
                self.state.image.mark_dirty();
                self.image_dirty = true;
            }
            let can_down = self.state.image.active_layer > 0;
            if ui.add_enabled(can_down, egui::Button::new("↓")).on_hover_text("Move down").clicked() {
                let idx = self.state.image.active_layer;
                self.state.image.layers.swap(idx, idx - 1);
                self.state.image.active_layer = idx - 1;
                self.state.image.mark_dirty();
                self.image_dirty = true;
            }
            if ui.button("⎘").on_hover_text("Duplicate layer").clicked() {
                let idx = self.state.image.active_layer;
                let clone = self.state.image.layers[idx].clone();
                self.state.image.layers.insert(idx + 1, clone);
                self.state.image.active_layer = idx + 1;
                self.state.image.mark_dirty();
                self.image_dirty = true;
            }
            let can_merge = self.state.image.active_layer > 0;
            if ui.add_enabled(can_merge, egui::Button::new("⇩")).on_hover_text("Merge down").clicked() {
                self.state.image.merge_down();
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
                        egui::ComboBox::from_id_source(format!("blend_{}", idx))
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

        ui.separator();

        // HSV color wheel
        egui::CollapsingHeader::new("Color wheel")
            .default_open(self.show_color_wheel)
            .show(ui, |ui| {
                self.show_color_wheel = true;
                let c1 = self.state.color1;
                let mut c1_32 = Color32::from_rgba_unmultiplied(c1[0], c1[1], c1[2], 255);
                if egui::color_picker::color_picker_color32(
                    ui,
                    &mut c1_32,
                    egui::color_picker::Alpha::Opaque,
                ) {
                    let [r, g, b, _] = c1_32.to_array();
                    self.state.color1 = Rgba([r, g, b, 255]);
                }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Color 2:").small());
                    let c2 = self.state.color2;
                    let mut c2_arr = [c2[0], c2[1], c2[2]];
                    if ui.color_edit_button_srgb(&mut c2_arr).changed() {
                        self.state.color2 = Rgba([c2_arr[0], c2_arr[1], c2_arr[2], 255]);
                    }
                    if ui.small_button("⇄").on_hover_text("Swap").clicked() {
                        std::mem::swap(&mut self.state.color1, &mut self.state.color2);
                    }
                });
            });

        ui.separator();

        // Brush controls
        ui.horizontal(|ui| {
            ui.label("Opacity");
            ui.add(egui::Slider::new(&mut self.state.tool_settings.brush_opacity, 0.01..=1.0).show_value(false));
            ui.label(format!("{:.0}%", self.state.tool_settings.brush_opacity * 100.0));
        });
        ui.horizontal(|ui| {
            ui.label("Size");
            ui.add(egui::Slider::new(&mut self.state.tool_settings.brush_size, 1.0..=200.0).show_value(false));
            ui.label(format!("{:.0}px", self.state.tool_settings.brush_size));
        });
        ui.horizontal(|ui| {
            ui.label("Smooth");
            ui.add(egui::Slider::new(&mut self.state.tool_settings.brush_stabilization, 0.0..=0.95).show_value(false));
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
        self.state.active_tool.active_shape_kind()
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
            "Pencil"  => Box::new(PencilTool::new(w, h)),
            "Eraser"  => Box::new(EraserTool::new(w, h)),
            "Brush"   => Box::new(BrushTool::new(w, h)),
            "Fill"    => Box::new(FillTool::new()),
            "Line"    => Box::new(LineTool::new(w, h)),
            "Smear"   => Box::new(SmearTool::new(w, h)),
            "Airbrush"=> Box::new(AirbrushTool::new(w, h)),
            _ => Box::new(PencilTool::new(w, h)),
        };
    }

    fn stamp_floating_selection(&mut self) {
        if let Some(fs) = self.state.floating_selection.take() {
            let x = fs.pos.x as i32;
            let y = fs.pos.y as i32;
            let iw = self.state.image.width() as i32;
            let ih = self.state.image.height() as i32;
            if let Some(buf) = self.state.image.get_active_raster_buffer_mut() {
                for py in 0..fs.image.height() as i32 {
                    for px in 0..fs.image.width() as i32 {
                        let tx = x + px;
                        let ty = y + py;
                        if tx >= 0 && tx < iw && ty >= 0 && ty < ih {
                            use image::GenericImageView;
                            let p = *fs.image.get_pixel(px as u32, py as u32);
                            if p[3] > 0 { buf.put_pixel(tx as u32, ty as u32, p); }
                        }
                    }
                }
            }
            self.state.image.mark_dirty();
            self.image_dirty = true;
        }
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

        if self.show_layers_panel {
            egui::SidePanel::right("layers_panel").min_width(180.0).show(ctx, |ui| {
                self.render_layers_panel(ui);
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
    }
}
