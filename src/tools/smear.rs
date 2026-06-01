use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::base::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub struct SmearTool {
    layer: RgbaImage,
    carry: Option<Rgba<u8>>,
    last_pos: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl SmearTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layer: ImageBuffer::new(width, height),
            carry: None,
            last_pos: None,
            dirty_rect: None,
        }
    }

    fn sample_avg(src: &RgbaImage, cx: f32, cy: f32, radius: f32) -> Option<Rgba<u8>> {
        let r = radius as i32;
        let w = src.width() as i32;
        let h = src.height() as i32;
        let (mut ra, mut ga, mut ba, mut aa, mut count) = (0u32, 0u32, 0u32, 0u32, 0u32);
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r { continue; }
                let px = cx as i32 + dx;
                let py = cy as i32 + dy;
                if px < 0 || px >= w || py < 0 || py >= h { continue; }
                let p = src.get_pixel(px as u32, py as u32);
                ra += p[0] as u32;
                ga += p[1] as u32;
                ba += p[2] as u32;
                aa += p[3] as u32;
                count += 1;
            }
        }
        if count == 0 { return None; }
        Some(Rgba([(ra/count) as u8, (ga/count) as u8, (ba/count) as u8, (aa/count) as u8]))
    }

    fn mix_colors(a: Rgba<u8>, b: Rgba<u8>, t: f32) -> Rgba<u8> {
        let lerp = |x: u8, y: u8| -> u8 { (x as f32 * (1.0 - t) + y as f32 * t) as u8 };
        Rgba([lerp(a[0], b[0]), lerp(a[1], b[1]), lerp(a[2], b[2]), lerp(a[3], b[3])])
    }

    fn paint_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Rgba<u8>) {
        let r = radius as i32;
        let r_sq = r * r;
        let w = self.layer.width() as i32;
        let h = self.layer.height() as i32;
        let min_x = (cx as i32 - r).max(0);
        let max_x = (cx as i32 + r).min(w - 1);
        let min_y = (cy as i32 - r).max(0);
        let max_y = (cy as i32 + r).min(h - 1);

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px - cx as i32;
                let dy = py - cy as i32;
                if dx * dx + dy * dy <= r_sq {
                    self.layer.put_pixel(px as u32, py as u32, color);
                    let rect = Rect::from_min_size(Pos2::new(px as f32, py as f32), Vec2::splat(1.0));
                    self.dirty_rect = Some(match self.dirty_rect { Some(d) => d.union(rect), None => rect });
                }
            }
        }
    }
}

impl Tool for SmearTool {
    fn name(&self) -> &str { "Smear" }

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
                let size = settings.brush_size;

                // Bootstrap carry color from the composite on first touch
                if self.carry.is_none() {
                    let composite = image.get_composite();
                    self.carry = Self::sample_avg(composite, pos.x, pos.y, size);
                }

                let carry = self.carry.unwrap_or(Rgba([0, 0, 0, 255]));

                // If we moved, sample the layer at the new spot and blend carry into it
                if self.last_pos.is_some() {
                    // Sample what's already on the layer at new pos (or composite for first look)
                    let local = {
                        let composite = image.get_composite();
                        Self::sample_avg(composite, pos.x, pos.y, size * 0.5)
                            .unwrap_or(carry)
                    };
                    // Carry momentum: 85% old carry, 15% pickup from canvas
                    let new_carry = Self::mix_colors(carry, local, 0.15);
                    self.carry = Some(new_carry);
                    self.paint_circle(pos.x, pos.y, size, new_carry);
                } else {
                    self.paint_circle(pos.x, pos.y, size, carry);
                }

                self.last_pos = Some(pos);
            }
        } else {
            self.last_pos = None;
            self.carry = None;
        }

        if input.is_released {
            if let Some(rect) = self.dirty_rect {
                let x = rect.min.x as u32;
                let y = rect.min.y as u32;
                let w = (rect.width() as u32 + 1).min(image.width().saturating_sub(x));
                let h = (rect.height() as u32 + 1).min(image.height().saturating_sub(y));
                let layer_index = image.active_layer;

                if let Some(layer) = image.layers.get_mut(layer_index) {
                    if let crate::layers::LayerData::Raster(ref mut target)
                    | crate::layers::LayerData::Tone { buffer: ref mut target, .. } = layer.data
                    {
                        if w > 0 && h > 0 {
                            let old_patch = target.view(x, y, w, h).to_image();
                            let src = self.layer.view(x, y, w, h).to_image();
                            for py in 0..h {
                                for px in 0..w {
                                    let p = src.get_pixel(px, py);
                                    if p[3] > 0 {
                                        target.put_pixel(x + px, y + py, *p);
                                        self.layer.put_pixel(x + px, y + py, Rgba([0, 0, 0, 0]));
                                    }
                                }
                            }
                            let new_patch = target.view(x, y, w, h).to_image();
                            image.mark_dirty();
                            self.dirty_rect = None;
                            return Some(Box::new(PatchCommand {
                                name: "Smear".to_string(),
                                layer_index, x, y, old_patch, new_patch,
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

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, settings: &crate::state::ToolSettings, pos: Pos2) {
        let carry_color = self.carry
            .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 200))
            .unwrap_or(Color32::WHITE);
        painter.circle_stroke(pos, settings.brush_size, egui::Stroke::new(2.0, carry_color));
        painter.circle_stroke(pos, settings.brush_size, egui::Stroke::new(0.5, Color32::BLACK));
    }

    fn configure(&mut self, _ui: &mut Ui, _settings: &mut crate::state::ToolSettings) {}
}
