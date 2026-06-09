mod canvas;
mod colors_panel;
mod dialogs;
mod history_panel;
mod layers_panel;
mod menu_bar;
mod status_bar;
mod theme;
mod toolbar;
mod tools_panel;
use crate::image_store::ImageStore;
use crate::state::{AppState, FloatingSelection};
use crate::tools::{
    AirbrushTool, BrushTool, EraserTool, FillTool,
    LineTool, PencilTool, SmearTool,
};
use eframe::egui::{
    self, Context, Pos2, TextureOptions, Vec2,
};
use eframe::Frame;
use image::GenericImageView;

pub struct ArsApp {
    state: AppState,
    base_texture: Option<egui::TextureHandle>,
    layer_texture: Option<egui::TextureHandle>,
    selection_texture: Option<egui::TextureHandle>,
    logo_texture: Option<egui::TextureHandle>,
    zoom: f32,
    pan: Vec2,
    image_dirty: bool,
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
    // eyedropper alt-mode
    alt_eyedropper_active: bool,
    pre_alt_tool_name: String,
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
            logo_texture: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            image_dirty: true,
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

        if self.logo_texture.is_none() {
            if let Ok(img) = image::load_from_memory(include_bytes!("../logo.png")) {
                let rgba = img.to_rgba8();
                let ci = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                self.logo_texture = Some(ctx.load_texture("logo", ci, TextureOptions::NEAREST));
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

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
        let Some(fs) = self.state.floating_selection.take() else { return; };
        let cmd = self.state.image.edit("Paste", |img| {
            let (iw, ih) = (img.width() as i32, img.height() as i32);
            if let Some(buf) = img.get_active_raster_buffer_mut() {
                use image::GenericImageView;
                for py in 0..fs.image.height() as i32 {
                    for px in 0..fs.image.width() as i32 {
                        let (tx, ty) = (fs.pos.x as i32 + px, fs.pos.y as i32 + py);
                        if tx >= 0 && tx < iw && ty >= 0 && ty < ih {
                            let p = *fs.image.get_pixel(px as u32, py as u32);
                            if p[3] > 0 { buf.put_pixel(tx as u32, ty as u32, p); }
                        }
                    }
                }
            }
        });
        self.state.command_stack.push(cmd);
        self.image_dirty = true;
    }

    fn do_select_all(&mut self) {
        let (w, h) = (self.state.image.width(), self.state.image.height());
        let mut mask = image::GrayImage::new(w, h);
        for p in mask.pixels_mut() { *p = image::Luma([255]); }
        self.state.image.selection = Some(mask);
    }

    fn do_copy(&mut self) {
        let Some(buf) = self.state.image.get_active_raster_snapshot() else { return; };
        if let Some(mask) = &self.state.image.selection {
            let mut min_x = u32::MAX; let mut min_y = u32::MAX; let mut max_x = 0u32; let mut max_y = 0u32;
            for (x, y, p) in mask.enumerate_pixels() {
                if p[0] > 0 { min_x = min_x.min(x); min_y = min_y.min(y); max_x = max_x.max(x); max_y = max_y.max(y); }
            }
            if max_x < min_x || max_y < min_y { return; }
            let (w, h) = (max_x - min_x + 1, max_y - min_y + 1);
            let mut out = image::RgbaImage::new(w, h);
            for y in 0..h {
                for x in 0..w {
                    let (sx, sy) = (min_x + x, min_y + y);
                    if mask.get_pixel(sx, sy)[0] > 0 {
                        out.put_pixel(x, y, *buf.get_pixel(sx, sy));
                    }
                }
            }
            self.state.clipboard = Some(out);
        } else {
            self.state.clipboard = Some(buf);
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
        let Some(mask) = self.state.image.selection.clone() else { return; };
        let cmd = self.state.image.edit("Delete", |img| {
            if let Some(buf) = img.get_active_raster_buffer_mut() {
                for (x, y, p) in mask.enumerate_pixels() {
                    if p[0] > 0 { buf.put_pixel(x, y, image::Rgba([0, 0, 0, 0])); }
                }
            }
        });
        self.state.command_stack.push(cmd);
        self.image_dirty = true;
    }

    fn do_crop(&mut self) {
        if let Some(mask) = &self.state.image.selection.clone() {
            let mut min_x = u32::MAX; let mut min_y = u32::MAX;
            let mut max_x = 0u32; let mut max_y = 0u32;
            for (x, y, p) in mask.enumerate_pixels() {
                if p[0] > 0 { min_x = min_x.min(x); min_y = min_y.min(y); max_x = max_x.max(x); max_y = max_y.max(y); }
            }
            if max_x >= min_x && max_y >= min_y {
                let (x, y, w, h) = (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1);
                let cmd = self.state.image.edit("Crop", |img| img.crop_to(x, y, w, h));
                self.state.command_stack.push(cmd);
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

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| { self.render_menu_bar(ui); });
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| { self.render_toolbar(ui); });

        if self.show_status_bar {
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| { self.render_status_bar(ui); });
        }

        egui::SidePanel::left("tools_panel").resizable(false).exact_width(104.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| { self.render_tools_panel(ui); });
        });

        if self.show_layers_panel {
            egui::SidePanel::right("right_dock").min_width(188.0).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::CollapsingHeader::new("Colors").default_open(true).show(ui, |ui| { self.render_colors_panel(ui); });
                    egui::CollapsingHeader::new("Layers").default_open(true).show(ui, |ui| { self.render_layers_panel(ui); });
                    egui::CollapsingHeader::new("History").default_open(true).show(ui, |ui| { self.render_history_panel(ui); });
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| { self.render_canvas(ui); });
    }
}
