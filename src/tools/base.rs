use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use egui::{Color32, Painter, Pos2, Rect, Ui};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub struct ToolInput {
    pub pos: Option<Pos2>, // Image space coordinates
    pub is_pressed: bool,
    pub is_released: bool,
}

pub trait Tool {
    fn name(&self) -> &str;

    fn update(
        &mut self,
        image: &mut ImageStore,
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>>;

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)>;

    fn draw_cursor(&self, ui: &mut Ui, painter: &Painter, pos: Pos2);

    // Draw tool specific settings
    fn configure(&mut self, ui: &mut Ui);
}

pub struct BrushTool {
    pub size: f32,
    layer: RgbaImage,
    last_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl BrushTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: 5.0,
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

    fn draw_segment(&mut self, start: Pos2, end: Pos2, color: Rgba<u8>) {
        let dist = start.distance(end);
        let steps = (dist / 1.0).max(1.0) as u32; // 1 pixel stepping for smoothness

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            self.draw_circle(pos, color);
        }
    }

    fn draw_circle(&mut self, pos: Pos2, color: Rgba<u8>) {
        let x = pos.x as i32;
        let y = pos.y as i32;
        let r = self.size as i32;
        let r_sq = r * r;

        let width = self.layer.width() as i32;
        let height = self.layer.height() as i32;

        let min_x = (x - r).max(0);
        let max_x = (x + r).min(width - 1);
        let min_y = (y - r).max(0);
        let max_y = (y + r).min(height - 1);

        // Track dirty area
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
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        // Handle resize if needed
        if self.layer.width() != image.width() || self.layer.height() != image.height() {
            self.layer = ImageBuffer::new(image.width(), image.height());
        }

        if input.is_pressed {
            if let Some(pos) = input.pos {
                if let Some(last) = self.last_pos {
                    self.draw_segment(last, pos, color);
                } else {
                    self.draw_circle(pos, color);
                }
                self.last_pos = Some(pos);
            }
        } else {
            self.last_pos = None;
        }

        if input.is_released {
            if let Some(rect) = self.dirty_rect {
                // Commit changes
                // 1. Capture old patch from image
                let x = rect.min.x as u32;
                let y = rect.min.y as u32;
                let w = rect.width() as u32;
                let h = rect.height() as u32;

                // Clamp to image bounds to be safe
                let w = w.min(image.width() - x);
                let h = h.min(image.height() - y);

                if w > 0 && h > 0 {
                    let old_patch = image.buffer.view(x, y, w, h).to_image();

                    // 2. Blend layer onto image
                    let layer_patch = self.layer.view(x, y, w, h).to_image();

                    // Manual blend and clear
                    for ly in 0..h {
                        for lx in 0..w {
                            let pixel = layer_patch.get_pixel(lx, ly);
                            if pixel[3] > 0 {
                                // Standard Alpha blending
                                // For Brush, we overwrite or blend.
                                // image.buffer.put_pixel(x + lx, y + ly, *pixel);
                                // Actually, `put_pixel` just sets it.
                                // To blend, we need to read background.
                                // Simplest: just set (Painter's algorithm)
                                image.buffer.put_pixel(x + lx, y + ly, *pixel);

                                // Clear pixel in layer
                                self.layer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                            }
                        }
                    }

                    // 3. Capture new patch
                    let new_patch = image.buffer.view(x, y, w, h).to_image();

                    self.dirty_rect = None;

                    return Some(Box::new(PatchCommand {
                        name: "Brush Stroke".to_string(),
                        x,
                        y,
                        old_patch,
                        new_patch,
                    }));
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

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, pos: Pos2) {
        painter.circle_stroke(pos, self.size, egui::Stroke::new(1.0, Color32::WHITE));
    }

    fn configure(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut self.size).range(1.0..=100.0));
        });
    }
}

pub struct EraserTool {
    pub size: f32,
    layer: RgbaImage, // We draw "Mask" on this layer
    last_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl EraserTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: 10.0,
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

    fn draw_segment(&mut self, start: Pos2, end: Pos2) {
        let dist = start.distance(end);
        let steps = (dist / 1.0).max(1.0) as u32;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            self.draw_circle(pos);
        }
    }

    fn draw_circle(&mut self, pos: Pos2) {
        let x = pos.x as i32;
        let y = pos.y as i32;
        let r = self.size as i32;
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

        // Draw "White" on the layer to indicate erasing area
        let color = Rgba([255, 255, 255, 128]); // Semi-transparent white for preview

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
        input: &ToolInput,
        _color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if self.layer.width() != image.width() || self.layer.height() != image.height() {
            self.layer = ImageBuffer::new(image.width(), image.height());
        }

        if input.is_pressed {
            if let Some(pos) = input.pos {
                if let Some(last) = self.last_pos {
                    self.draw_segment(last, pos);
                } else {
                    self.draw_circle(pos);
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

                if w > 0 && h > 0 {
                    let old_patch = image.buffer.view(x, y, w, h).to_image();
                    let layer_patch = self.layer.view(x, y, w, h).to_image();

                    for ly in 0..h {
                        for lx in 0..w {
                            let pixel = layer_patch.get_pixel(lx, ly);
                            if pixel[3] > 0 {
                                // If layer has opacity, we ERASE the target
                                image.buffer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                                self.layer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                            }
                        }
                    }

                    let new_patch = image.buffer.view(x, y, w, h).to_image();
                    self.dirty_rect = None;

                    return Some(Box::new(PatchCommand {
                        name: "Erase".to_string(),
                        x,
                        y,
                        old_patch,
                        new_patch,
                    }));
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

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, pos: Pos2) {
        painter.circle_stroke(pos, self.size, egui::Stroke::new(1.0, Color32::RED));
    }

    fn configure(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut self.size).range(1.0..=100.0));
        });
    }
}

pub struct LineTool {
    pub width: f32,
    layer: RgbaImage,
    start_pos: Option<Pos2>,
    current_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl LineTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: 2.0,
            layer: ImageBuffer::new(width, height),
            start_pos: None,
            current_pos: None,
            dirty_rect: None,
        }
    }

    fn draw_line_on_layer(&mut self, start: Pos2, end: Pos2, color: Rgba<u8>) {
        // First clear the previous line if any
        // Since we don't track the *previous* line rect exactly in this simple impl,
        // we might need to clear the whole dirty rect.
        // Or cleaner: Reset layer for the dirty rect.
        // For efficiency, we can just clear the whole layer (slow) or use the dirty rect.
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

        // Draw new line
        // Simple Bresenham with width?
        // Let's use the same "draw_circle" stepping approach for width
        let dist = start.distance(end);
        let steps = (dist / 1.0).max(1.0) as u32;

        let mut new_dirty: Option<Rect> = None;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);

            let x = pos.x as i32;
            let y = pos.y as i32;
            let r = self.width as i32;
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
                    self.draw_line_on_layer(start, pos, color);
                }
            }
        }

        if input.is_released {
            if let (Some(_start), Some(_end), Some(rect)) =
                (self.start_pos, self.current_pos, self.dirty_rect)
            {
                // Commit
                let x = rect.min.x as u32;
                let y = rect.min.y as u32;
                let w = rect.width() as u32;
                let h = rect.height() as u32;
                let w = w.min(image.width() - x);
                let h = h.min(image.height() - y);

                if w > 0 && h > 0 {
                    let old_patch = image.buffer.view(x, y, w, h).to_image();
                    let layer_patch = self.layer.view(x, y, w, h).to_image();

                    for ly in 0..h {
                        for lx in 0..w {
                            let pixel = layer_patch.get_pixel(lx, ly);
                            if pixel[3] > 0 {
                                image.buffer.put_pixel(x + lx, y + ly, *pixel);
                                self.layer.put_pixel(x + lx, y + ly, Rgba([0, 0, 0, 0]));
                            }
                        }
                    }

                    let new_patch = image.buffer.view(x, y, w, h).to_image();
                    self.start_pos = None;
                    self.current_pos = None;
                    self.dirty_rect = None;

                    return Some(Box::new(PatchCommand {
                        name: "Line".to_string(),
                        x,
                        y,
                        old_patch,
                        new_patch,
                    }));
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

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, pos: Pos2) {
        painter.circle_filled(pos, 2.0, Color32::WHITE);
    }

    fn configure(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Width:");
            ui.add(egui::DragValue::new(&mut self.width).range(1.0..=20.0));
        });
    }
}
