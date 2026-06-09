use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::base::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use std::collections::VecDeque;

pub struct AirbrushTool {
    layer: RgbaImage,
    dirty_rect: Option<Rect>,
    rng_state: u64,
}

impl AirbrushTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layer: ImageBuffer::new(width, height),
            dirty_rect: None,
            rng_state: 12345,
        }
    }

    fn next_rand(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    fn spray(&mut self, cx: f32, cy: f32, radius: f32, color: Rgba<u8>, density: u32) {
        let w = self.layer.width() as i32;
        let h = self.layer.height() as i32;
        for _ in 0..density {
            let angle = (self.next_rand() % 1000) as f32 / 1000.0 * std::f32::consts::TAU;
            let r = ((self.next_rand() % 1000) as f32 / 1000.0).sqrt() * radius;
            let x = (cx + r * angle.cos()) as i32;
            let y = (cy + r * angle.sin()) as i32;
            if x >= 0 && x < w && y >= 0 && y < h {
                self.layer.put_pixel(x as u32, y as u32, color);
                let pt = Rect::from_min_size(Pos2::new(x as f32, y as f32), Vec2::splat(1.0));
                self.dirty_rect = Some(match self.dirty_rect { Some(d) => d.union(pt), None => pt });
            }
        }
    }
}

impl Tool for AirbrushTool {
    fn name(&self) -> &str { "Airbrush" }

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
            if let Some(pos) = input.pos {
                self.spray(pos.x, pos.y, settings.airbrush_radius, color, 30);
            }
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
                    let target = &mut layer.pixels;
                    {
                        if w > 0 && h > 0 {
                            use image::GenericImageView;
                            let old_patch = target.view(x, y, w, h).to_image();
                            let src = self.layer.view(x, y, w, h).to_image();
                            for py in 0..h {
                                for px in 0..w {
                                    let p = src.get_pixel(px, py);
                                    if p[3] > 0 {
                                        let in_sel = selection.as_ref()
                                            .map(|m| m.get_pixel(x+px, y+py)[0] > 0)
                                            .unwrap_or(true);
                                        if in_sel { target.put_pixel(x+px, y+py, *p); }
                                        self.layer.put_pixel(x+px, y+py, Rgba([0,0,0,0]));
                                    }
                                }
                            }
                            let new_patch = target.view(x, y, w, h).to_image();
                            image.mark_dirty();
                            self.dirty_rect = None;
                            return Some(Box::new(PatchCommand {
                                name: "Airbrush".to_string(),
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
        painter.circle_stroke(pos, settings.airbrush_radius, egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255,255,255,120)));
    }

    fn configure(&mut self, ui: &mut Ui, settings: &mut crate::state::ToolSettings) {
        ui.horizontal(|ui| {
            ui.label("Radius:");
            ui.add(egui::DragValue::new(&mut settings.airbrush_radius).range(5.0..=100.0));
        });
    }
}
