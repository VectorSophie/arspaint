use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub struct RectangleTool {
    layer: RgbaImage,
    start_pos: Option<Pos2>,
    current_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl RectangleTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layer: ImageBuffer::new(width, height),
            start_pos: None,
            current_pos: None,
            dirty_rect: None,
        }
    }

    fn draw_rect_on_layer(&mut self, start: Pos2, end: Pos2, color: Rgba<u8>, width: f32) {
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

        let min_x = start.x.min(end.x);
        let max_x = start.x.max(end.x);
        let min_y = start.y.min(end.y);
        let max_y = start.y.max(end.y);

        let tl = Pos2::new(min_x, min_y);
        let tr = Pos2::new(max_x, min_y);
        let br = Pos2::new(max_x, max_y);
        let bl = Pos2::new(min_x, max_y);

        let mut new_dirty: Option<Rect> = None;

        let mut draw_line =
            |p1: Pos2, p2: Pos2, layer: &mut RgbaImage, dirty: &mut Option<Rect>| {
                let dist = p1.distance(p2);
                let steps = (dist / 1.0).max(1.0) as u32;

                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let pos = p1.lerp(p2, t);

                    let x = pos.x as i32;
                    let y = pos.y as i32;
                    let r = width as i32;
                    let r_sq = r * r;

                    let width_img = layer.width() as i32;
                    let height_img = layer.height() as i32;

                    let min_x = (x - r).max(0);
                    let max_x = (x + r).min(width_img - 1);
                    let min_y = (y - r).max(0);
                    let max_y = (y + r).min(height_img - 1);

                    let rect = Rect::from_min_max(
                        Pos2::new(min_x as f32, min_y as f32),
                        Pos2::new(max_x as f32 + 1.0, max_y as f32 + 1.0),
                    );
                    *dirty = Some(match *dirty {
                        Some(r) => r.union(rect),
                        None => rect,
                    });

                    for cy in min_y..=max_y {
                        for cx in min_x..=max_x {
                            if (cx - x) * (cx - x) + (cy - y) * (cy - y) <= r_sq {
                                layer.put_pixel(cx as u32, cy as u32, color);
                            }
                        }
                    }
                }
            };

        draw_line(tl, tr, &mut self.layer, &mut new_dirty);
        draw_line(tr, br, &mut self.layer, &mut new_dirty);
        draw_line(br, bl, &mut self.layer, &mut new_dirty);
        draw_line(bl, tl, &mut self.layer, &mut new_dirty);

        self.dirty_rect = new_dirty;
    }
}

impl Tool for RectangleTool {
    fn name(&self) -> &str {
        "Rectangle"
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
                    self.draw_rect_on_layer(start, pos, color, settings.line_width);
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
                            name: "Rectangle".to_string(),
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
        painter.circle_stroke(
            pos,
            settings.line_width,
            egui::Stroke::new(1.0, Color32::WHITE),
        );
    }

    fn configure(&mut self, ui: &mut Ui, settings: &mut crate::state::ToolSettings) {
        ui.horizontal(|ui| {
            ui.label("Width:");
            ui.add(egui::DragValue::new(&mut settings.line_width).range(1.0..=20.0));
        });
    }
}
