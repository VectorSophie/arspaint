use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub struct ToolInput {
    pub pos: Option<Pos2>,
    pub is_pressed: bool,
    pub is_released: bool,
}

pub trait Tool {
    fn name(&self) -> &str;

    fn update(
        &mut self,
        image: &mut ImageStore,
        settings: &crate::state::ToolSettings,
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>>;

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)>;

    fn draw_cursor(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        settings: &crate::state::ToolSettings,
        pos: Pos2,
    );

    fn configure(&mut self, ui: &mut Ui, settings: &mut crate::state::ToolSettings);
}

pub struct BrushTool {
    pub texture: Option<RgbaImage>,
    layer: RgbaImage,
    last_pos: Option<Pos2>,
    stabilized_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl BrushTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            texture: None,
            layer: ImageBuffer::new(width, height),
            last_pos: None,
            stabilized_pos: None,
            dirty_rect: None,
        }
    }

    fn expand_dirty_rect(&mut self, rect: Rect) {
        self.dirty_rect = Some(match self.dirty_rect {
            Some(r) => r.union(rect),
            None => rect,
        });
    }

    fn draw_segment(&mut self, start: Pos2, end: Pos2, color: Rgba<u8>, size: f32, spacing: f32) {
        let dist = start.distance(end);
        let step_dist = (size * spacing).max(1.0);
        let steps = (dist / step_dist).max(1.0) as u32;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            if self.texture.is_some() {
                self.draw_texture_stamp(pos, color, size);
            } else {
                self.draw_circle(pos, color, size);
            }
        }
    }

    fn draw_texture_stamp(&mut self, pos: Pos2, color: Rgba<u8>, size: f32) {
        if self.texture.is_none() {
            return;
        }
        let (tw, th) = self.texture.as_ref().unwrap().dimensions();
        let scale_x = size * 2.0 / tw as f32;
        let scale_y = size * 2.0 / th as f32;

        let start_x = (pos.x - size) as i32;
        let start_y = (pos.y - size) as i32;

        let width = self.layer.width() as i32;
        let height = self.layer.height() as i32;

        let rect = Rect::from_min_size(
            Pos2::new(start_x as f32, start_y as f32),
            Vec2::new(size * 2.0, size * 2.0),
        );
        self.expand_dirty_rect(rect);

        for sy in 0..(size * 2.0) as i32 {
            for sx in 0..(size * 2.0) as i32 {
                let tx = (sx as f32 / scale_x) as u32;
                let ty = (sy as f32 / scale_y) as u32;

                if tx < tw && ty < th {
                    let target_x = start_x + sx;
                    let target_y = start_y + sy;

                    if target_x >= 0 && target_x < width && target_y >= 0 && target_y < height {
                        let tex_pixel = self.texture.as_ref().unwrap().get_pixel(tx, ty);
                        let alpha = (tex_pixel[3] as f32 / 255.0) * (color[3] as f32 / 255.0);
                        if alpha > 0.0 {
                            let mut final_color = color;
                            final_color[3] = (alpha * 255.0) as u8;
                            let existing = self.layer.get_pixel(target_x as u32, target_y as u32);
                            if alpha > (existing[3] as f32 / 255.0) {
                                self.layer
                                    .put_pixel(target_x as u32, target_y as u32, final_color);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_circle(&mut self, pos: Pos2, color: Rgba<u8>, size: f32) {
        let x = pos.x as i32;
        let y = pos.y as i32;
        let r = size as i32;
        let r_sq = r * r;

        let width = self.layer.width() as i32;
        let height = self.layer.height() as i32;

        let min_x = (x - r).max(0);
        let max_x = (x + r).min(width - 1);
        let min_y = (y - r).max(0);
        let max_y = (y + r).min(height - 1);

        let rect = Rect::from_min_max(
            Pos2::new(min_x as f32, min_y as f32),
            Pos2::new(max_x as f32 + 1.0, max_y as f32 + 1.0),
        );
        self.expand_dirty_rect(rect);

        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                if (cx - x) * (cx - x) + (cy - y) * (cy - y) <= r_sq {
                    self.layer.put_pixel(cx as u32, cy as u32, color);
                }
            }
        }
    }
}

impl Tool for BrushTool {
    fn name(&self) -> &str {
        "Brush"
    }

    fn update(
        &mut self,
        image: &mut ImageStore,
        settings: &crate::state::ToolSettings,
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if self.layer.width() != image.width() || self.layer.height() != image.height() {
            self.layer = ImageBuffer::new(image.width(), image.height());
        }

        if input.is_pressed {
            if let Some(target_pos) = input.pos {
                let current_stabilized = if let Some(last_s) = self.stabilized_pos {
                    let weight = settings.brush_stabilization.clamp(0.0, 0.95);
                    let smoothed_x = last_s.x * weight + target_pos.x * (1.0 - weight);
                    let smoothed_y = last_s.y * weight + target_pos.y * (1.0 - weight);
                    Pos2::new(smoothed_x, smoothed_y)
                } else {
                    target_pos
                };

                if let Some(last) = self.last_pos {
                    self.draw_segment(
                        last,
                        current_stabilized,
                        color,
                        settings.brush_size,
                        settings.brush_spacing,
                    );
                } else {
                    if self.texture.is_some() {
                        self.draw_texture_stamp(current_stabilized, color, settings.brush_size);
                    } else {
                        self.draw_circle(current_stabilized, color, settings.brush_size);
                    }
                }

                self.last_pos = Some(current_stabilized);
                self.stabilized_pos = Some(current_stabilized);
            }
        } else {
            self.last_pos = None;
            self.stabilized_pos = None;
        }

        if input.is_released {
            if let Some(rect) = self.dirty_rect {
                let x = rect.min.x as u32;
                let y = rect.min.y as u32;
                let w = rect.width() as u32;
                let h = rect.height() as u32;
                let w = w.min(image.width() - x);
                let h = h.min(image.height() - y);

                let layer_index = image.active_layer;
                let alpha_locked = image.layers[layer_index].alpha_locked;

                if let Some(target_buffer) = image.get_active_raster_buffer_mut() {
                    if w > 0 && h > 0 {
                        let old_patch = target_buffer.view(x, y, w, h).to_image();
                        let layer_patch = self.layer.view(x, y, w, h).to_image();
                        for ly in 0..h {
                            for lx in 0..w {
                                let pixel = layer_patch.get_pixel(lx, ly);
                                if pixel[3] > 0 {
                                    let target_pixel = target_buffer.get_pixel(x + lx, y + ly);
                                    if !alpha_locked || target_pixel[3] > 0 {
                                        let mut final_pixel = *pixel;
                                        if alpha_locked {
                                            final_pixel[3] = target_pixel[3];
                                        }
                                        target_buffer.put_pixel(x + lx, y + ly, final_pixel);
                                    }
                                    self.layer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                                }
                            }
                        }
                        let new_patch = target_buffer.view(x, y, w, h).to_image();
                        image.mark_dirty();
                        self.dirty_rect = None;

                        return Some(Box::new(PatchCommand {
                            name: "Brush Stroke".to_string(),
                            layer_index,
                            x,
                            y,
                            old_patch,
                            new_patch,
                        }));
                    }
                }
            }
            self.dirty_rect = None;
        }

        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        if self.dirty_rect.is_some() {
            Some((&self.layer, 0, 0))
        } else {
            None
        }
    }

    fn draw_cursor(
        &self,
        _ui: &mut Ui,
        painter: &Painter,
        settings: &crate::state::ToolSettings,
        pos: Pos2,
    ) {
        painter.circle_stroke(
            pos,
            settings.brush_size,
            egui::Stroke::new(1.0, Color32::WHITE),
        );
    }

    fn configure(&mut self, ui: &mut Ui, settings: &mut crate::state::ToolSettings) {
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut settings.brush_size).range(1.0..=500.0));
            ui.label("Smoothing:");
            ui.add(egui::Slider::new(
                &mut settings.brush_stabilization,
                0.0..=0.95,
            ));
        });

        ui.horizontal(|ui| {
            ui.label("Spacing:");
            ui.add(egui::Slider::new(&mut settings.brush_spacing, 0.01..=2.0));

            if ui.button("Load Texture").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "bmp"])
                    .pick_file()
                {
                    if let Ok(img) = image::open(path) {
                        self.texture = Some(img.to_rgba8());
                    }
                }
            }
            if self.texture.is_some() && ui.button("Clear Texture").clicked() {
                self.texture = None;
            }
        });
    }
}

pub struct EraserTool {
    layer: RgbaImage,
    last_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl EraserTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layer: ImageBuffer::new(width, height),
            last_pos: None,
            dirty_rect: None,
        }
    }

    fn expand_dirty_rect(&mut self, rect: Rect) {
        self.dirty_rect = Some(match self.dirty_rect {
            Some(r) => r.union(rect),
            None => rect,
        });
    }

    fn draw_segment(&mut self, start: Pos2, end: Pos2, size: f32) {
        let dist = start.distance(end);
        let steps = (dist / 1.0).max(1.0) as u32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            self.draw_circle(pos, size);
        }
    }

    fn draw_circle(&mut self, pos: Pos2, size: f32) {
        let x = pos.x as i32;
        let y = pos.y as i32;
        let r = size as i32;
        let r_sq = r * r;
        let width = self.layer.width() as i32;
        let height = self.layer.height() as i32;
        let min_x = (x - r).max(0);
        let max_x = (x + r).min(width - 1);
        let min_y = (y - r).max(0);
        let max_y = (y + r).min(height - 1);

        let rect = Rect::from_min_max(
            Pos2::new(min_x as f32, min_y as f32),
            Pos2::new(max_x as f32 + 1.0, max_y as f32 + 1.0),
        );
        self.expand_dirty_rect(rect);

        let color = Rgba([255, 255, 255, 128]);
        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                if (cx - x) * (cx - x) + (cy - y) * (cy - y) <= r_sq {
                    self.layer.put_pixel(cx as u32, cy as u32, color);
                }
            }
        }
    }
}

impl Tool for EraserTool {
    fn name(&self) -> &str {
        "Eraser"
    }

    fn update(
        &mut self,
        image: &mut ImageStore,
        settings: &crate::state::ToolSettings,
        input: &ToolInput,
        _color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if self.layer.width() != image.width() || self.layer.height() != image.height() {
            self.layer = ImageBuffer::new(image.width(), image.height());
        }

        if input.is_pressed {
            if let Some(pos) = input.pos {
                if let Some(last) = self.last_pos {
                    self.draw_segment(last, pos, settings.eraser_size);
                } else {
                    self.draw_circle(pos, settings.eraser_size);
                }
                self.last_pos = Some(pos);
            }
        } else {
            self.last_pos = None;
        }

        if input.is_released {
            if let Some(rect) = self.dirty_rect {
                let x = rect.min.x as u32;
                let y = rect.min.y as u32;
                let w = rect.width() as u32;
                let h = rect.height() as u32;
                let w = w.min(image.width() - x);
                let h = h.min(image.height() - y);
                let layer_index = image.active_layer;
                let alpha_locked = image.layers[layer_index].alpha_locked;

                if let Some(target_buffer) = image.get_active_raster_buffer_mut() {
                    if w > 0 && h > 0 {
                        let old_patch = target_buffer.view(x, y, w, h).to_image();
                        let layer_patch = self.layer.view(x, y, w, h).to_image();

                        for ly in 0..h {
                            for lx in 0..w {
                                let pixel = layer_patch.get_pixel(lx, ly);
                                if pixel[3] > 0 {
                                    if !alpha_locked {
                                        target_buffer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                                    }
                                    self.layer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                                }
                            }
                        }
                        let new_patch = target_buffer.view(x, y, w, h).to_image();
                        image.mark_dirty();
                        self.dirty_rect = None;

                        return Some(Box::new(PatchCommand {
                            name: "Erase".to_string(),
                            layer_index,
                            x,
                            y,
                            old_patch,
                            new_patch,
                        }));
                    }
                }
            }
            self.dirty_rect = None;
        }
        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        if self.dirty_rect.is_some() {
            Some((&self.layer, 0, 0))
        } else {
            None
        }
    }

    fn draw_cursor(
        &self,
        _ui: &mut Ui,
        painter: &Painter,
        settings: &crate::state::ToolSettings,
        pos: Pos2,
    ) {
        painter.circle_stroke(
            pos,
            settings.eraser_size,
            egui::Stroke::new(1.0, Color32::RED),
        );
    }

    fn configure(&mut self, ui: &mut Ui, settings: &mut crate::state::ToolSettings) {
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut settings.eraser_size).range(1.0..=100.0));
        });
    }
}

pub struct LineTool {
    layer: RgbaImage,
    start_pos: Option<Pos2>,
    current_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl LineTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layer: ImageBuffer::new(width, height),
            start_pos: None,
            current_pos: None,
            dirty_rect: None,
        }
    }

    fn draw_line_on_layer(&mut self, start: Pos2, end: Pos2, color: Rgba<u8>, width: f32) {
        if let Some(rect) = self.dirty_rect {
            let x = rect.min.x as u32;
            let y = rect.min.y as u32;
            let w = rect.width() as u32;
            let h = rect.height() as u32;
            let w = w.min(self.layer.width() - x);
            let h = h.min(self.layer.height() - y);
            for ly in 0..h {
                for lx in 0..w {
                    self.layer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                }
            }
        }

        let dist = start.distance(end);
        let steps = (dist / 1.0).max(1.0) as u32;
        let mut new_dirty: Option<Rect> = None;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            let x = pos.x as i32;
            let y = pos.y as i32;
            let r = width as i32;
            let r_sq = r * r;
            let width_img = self.layer.width() as i32;
            let height_img = self.layer.height() as i32;
            let min_x = (x - r).max(0);
            let max_x = (x + r).min(width_img - 1);
            let min_y = (y - r).max(0);
            let max_y = (y + r).min(height_img - 1);

            let rect = Rect::from_min_max(
                Pos2::new(min_x as f32, min_y as f32),
                Pos2::new(max_x as f32 + 1.0, max_y as f32 + 1.0),
            );
            new_dirty = Some(match new_dirty {
                Some(r) => r.union(rect),
                None => rect,
            });

            for cy in min_y..=max_y {
                for cx in min_x..=max_x {
                    if (cx - x) * (cx - x) + (cy - y) * (cy - y) <= r_sq {
                        self.layer.put_pixel(cx as u32, cy as u32, color);
                    }
                }
            }
        }
        self.dirty_rect = new_dirty;
    }
}

impl Tool for LineTool {
    fn name(&self) -> &str {
        "Line"
    }

    fn update(
        &mut self,
        image: &mut ImageStore,
        settings: &crate::state::ToolSettings,
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if self.layer.width() != image.width() || self.layer.height() != image.height() {
            self.layer = ImageBuffer::new(image.width(), image.height());
        }

        if input.is_pressed {
            if self.start_pos.is_none() {
                self.start_pos = input.pos;
            }
            if let Some(pos) = input.pos {
                self.current_pos = Some(pos);
                if let Some(start) = self.start_pos {
                    self.draw_line_on_layer(start, pos, color, settings.line_width);
                }
            }
        }

        if input.is_released {
            if let (Some(_start), Some(_end), Some(rect)) =
                (self.start_pos, self.current_pos, self.dirty_rect)
            {
                let x = rect.min.x as u32;
                let y = rect.min.y as u32;
                let w = rect.width() as u32;
                let h = rect.height() as u32;
                let w = w.min(image.width() - x);
                let h = h.min(image.height() - y);
                let layer_index = image.active_layer;
                let alpha_locked = image.layers[layer_index].alpha_locked;

                if let Some(target_buffer) = image.get_active_raster_buffer_mut() {
                    if w > 0 && h > 0 {
                        let old_patch = target_buffer.view(x, y, w, h).to_image();
                        let layer_patch = self.layer.view(x, y, w, h).to_image();

                        for ly in 0..h {
                            for lx in 0..w {
                                let pixel = layer_patch.get_pixel(lx, ly);
                                if pixel[3] > 0 {
                                    let target_pixel = target_buffer.get_pixel(x + lx, y + ly);
                                    if !alpha_locked || target_pixel[3] > 0 {
                                        let mut final_pixel = *pixel;
                                        if alpha_locked {
                                            final_pixel[3] = target_pixel[3];
                                        }
                                        target_buffer.put_pixel(x + lx, y + ly, final_pixel);
                                    }
                                    self.layer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                                }
                            }
                        }

                        let new_patch = target_buffer.view(x, y, w, h).to_image();
                        image.mark_dirty();
                        self.start_pos = None;
                        self.current_pos = None;
                        self.dirty_rect = None;

                        return Some(Box::new(PatchCommand {
                            name: "Line".to_string(),
                            layer_index,
                            x,
                            y,
                            old_patch,
                            new_patch,
                        }));
                    }
                }
            }
            self.start_pos = None;
            self.current_pos = None;
            self.dirty_rect = None;
        }
        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        if self.dirty_rect.is_some() {
            Some((&self.layer, 0, 0))
        } else {
            None
        }
    }

    fn draw_cursor(
        &self,
        _ui: &mut Ui,
        painter: &Painter,
        settings: &crate::state::ToolSettings,
        pos: Pos2,
    ) {
        painter.circle_filled(pos, settings.line_width, Color32::WHITE);
    }

    fn configure(&mut self, ui: &mut Ui, settings: &mut crate::state::ToolSettings) {
        ui.horizontal(|ui| {
            ui.label("Width:");
            ui.add(egui::DragValue::new(&mut settings.line_width).range(1.0..=20.0));
        });
    }
}
