use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub struct EllipseTool {
    pub width: f32,
    layer: RgbaImage,
    start_pos: Option<Pos2>,
    current_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl EllipseTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: 2.0,
            layer: ImageBuffer::new(width, height),
            start_pos: None,
            current_pos: None,
            dirty_rect: None,
        }
    }

    fn draw_ellipse_on_layer(&mut self, start: Pos2, end: Pos2, color: Rgba<u8>) {
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

        let center_x = (start.x + end.x) / 2.0;
        let center_y = (start.y + end.y) / 2.0;
        let radius_x = (end.x - start.x).abs() / 2.0;
        let radius_y = (end.y - start.y).abs() / 2.0;

        let mut new_dirty: Option<Rect> = None;

        // Simple ellipse approximation by stepping angle
        // Circumference approx: 2 * pi * sqrt((a^2 + b^2) / 2)
        let circ =
            2.0 * std::f32::consts::PI * ((radius_x.powi(2) + radius_y.powi(2)) / 2.0).sqrt();
        let steps = circ.max(10.0) as u32;

        for i in 0..=steps {
            let t = (i as f32 / steps as f32) * 2.0 * std::f32::consts::PI;
            let x = center_x + radius_x * t.cos();
            let y = center_y + radius_y * t.sin();
            let pos = Pos2::new(x, y);

            // Draw "brush" at this point
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

impl Tool for EllipseTool {
    fn name(&self) -> &str {
        "Ellipse"
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
                    self.draw_ellipse_on_layer(start, pos, color);
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
                        name: "Ellipse".to_string(),
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
