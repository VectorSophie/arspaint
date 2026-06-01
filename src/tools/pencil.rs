use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::base::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub struct PencilTool {
    layer: RgbaImage,
    last_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl PencilTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layer: ImageBuffer::new(width, height),
            last_pos: None,
            dirty_rect: None,
        }
    }

    fn plot(&mut self, x: i32, y: i32, color: Rgba<u8>) {
        let w = self.layer.width() as i32;
        let h = self.layer.height() as i32;
        if x >= 0 && x < w && y >= 0 && y < h {
            self.layer.put_pixel(x as u32, y as u32, color);
            let r = Rect::from_min_size(
                Pos2::new(x as f32, y as f32),
                egui::Vec2::splat(1.0),
            );
            self.dirty_rect = Some(match self.dirty_rect {
                Some(d) => d.union(r),
                None => r,
            });
        }
    }

    fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgba<u8>) {
        // Bresenham
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx: i32 = if x0 < x1 { 1 } else { -1 };
        let sy: i32 = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.plot(x, y, color);
            if x == x1 && y == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; x += sx; }
            if e2 <= dx { err += dx; y += sy; }
        }
    }
}

impl Tool for PencilTool {
    fn name(&self) -> &str { "Pencil" }

    fn update(
        &mut self,
        image: &mut ImageStore,
        _settings: &crate::state::ToolSettings,
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if self.layer.width() != image.width() || self.layer.height() != image.height() {
            self.layer = ImageBuffer::new(image.width(), image.height());
        }

        if input.is_pressed {
            if let Some(pos) = input.pos {
                let (x1, y1) = (pos.x as i32, pos.y as i32);
                if let Some(last) = self.last_pos {
                    self.draw_line(last.x as i32, last.y as i32, x1, y1, color);
                } else {
                    self.plot(x1, y1, color);
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
                let w = (rect.width() as u32).max(1).min(image.width().saturating_sub(x));
                let h = (rect.height() as u32).max(1).min(image.height().saturating_sub(y));
                let layer_index = image.active_layer;
                let selection = &image.selection;

                if let Some(layer) = image.layers.get_mut(layer_index) {
                    if let crate::layers::LayerData::Raster(ref mut target) | crate::layers::LayerData::Tone { buffer: ref mut target, .. } = layer.data {
                        if w > 0 && h > 0 {
                            let old_patch = target.view(x, y, w, h).to_image();
                            let src_patch = self.layer.view(x, y, w, h).to_image();
                            for py in 0..h {
                                for px in 0..w {
                                    let p = src_patch.get_pixel(px, py);
                                    if p[3] > 0 {
                                        let in_sel = selection.as_ref()
                                            .map(|m| m.get_pixel(x + px, y + py)[0] > 0)
                                            .unwrap_or(true);
                                        if in_sel {
                                            target.put_pixel(x + px, y + py, *p);
                                        }
                                        self.layer.put_pixel(x + px, y + py, Rgba([0,0,0,0]));
                                    }
                                }
                            }
                            let new_patch = target.view(x, y, w, h).to_image();
                            image.mark_dirty();
                            self.dirty_rect = None;
                            return Some(Box::new(PatchCommand {
                                name: "Pencil".to_string(),
                                layer_index,
                                x, y, old_patch, new_patch,
                            }));
                        }
                    }
                }
            }
            self.dirty_rect = None;
        }
        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        self.dirty_rect.map(|_| (&self.layer, 0, 0))
    }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _settings: &crate::state::ToolSettings, pos: Pos2) {
        painter.circle_filled(pos, 2.0, Color32::from_rgba_unmultiplied(128, 128, 128, 180));
    }

    fn configure(&mut self, _ui: &mut Ui, _settings: &mut crate::state::ToolSettings) {}
}
