use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::base::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

#[derive(PartialEq)]
enum CurvePhase { Idle, DrawingLine, BendingFirst, BendingSecond }

pub struct CurveTool {
    layer: RgbaImage,
    dirty_rect: Option<Rect>,
    phase: CurvePhase,
    p0: Pos2,
    p3: Pos2,
    p1: Pos2,
    p2: Pos2,
    color: Rgba<u8>,
    line_width: f32,
}

impl CurveTool {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layer: ImageBuffer::new(width, height),
            dirty_rect: None,
            phase: CurvePhase::Idle,
            p0: Pos2::ZERO, p3: Pos2::ZERO,
            p1: Pos2::ZERO, p2: Pos2::ZERO,
            color: Rgba([0,0,0,255]),
            line_width: 2.0,
        }
    }

    fn cubic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, steps: u32) -> Vec<Pos2> {
        (0..=steps).map(|i| {
            let t = i as f32 / steps as f32;
            let mt = 1.0 - t;
            Pos2::new(
                mt*mt*mt*p0.x + 3.0*mt*mt*t*p1.x + 3.0*mt*t*t*p2.x + t*t*t*p3.x,
                mt*mt*mt*p0.y + 3.0*mt*mt*t*p1.y + 3.0*mt*t*t*p2.y + t*t*t*p3.y,
            )
        }).collect()
    }

    fn redraw_layer(&mut self) {
        // clear previous dirty area
        if let Some(rect) = self.dirty_rect {
            let x = rect.min.x as u32;
            let y = rect.min.y as u32;
            let w = (rect.width() as u32 + 1).min(self.layer.width().saturating_sub(x));
            let h = (rect.height() as u32 + 1).min(self.layer.height().saturating_sub(y));
            for py in 0..h { for px in 0..w { self.layer.put_pixel(x+px, y+py, Rgba([0,0,0,0])); } }
        }
        self.dirty_rect = None;

        let steps = (self.p0.distance(self.p3) as u32 * 4).max(64);
        let pts = Self::cubic_bezier(self.p0, self.p1, self.p2, self.p3, steps);
        let r = self.line_width as i32;
        let r_sq = r * r;
        let w = self.layer.width() as i32;
        let h = self.layer.height() as i32;

        for pt in &pts {
            let cx = pt.x as i32;
            let cy = pt.y as i32;
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx*dx + dy*dy <= r_sq {
                        let px = cx + dx;
                        let py = cy + dy;
                        if px >= 0 && px < w && py >= 0 && py < h {
                            self.layer.put_pixel(px as u32, py as u32, self.color);
                            let rect = Rect::from_min_size(Pos2::new(px as f32, py as f32), Vec2::splat(1.0));
                            self.dirty_rect = Some(match self.dirty_rect { Some(d) => d.union(rect), None => rect });
                        }
                    }
                }
            }
        }
    }

    fn commit(&mut self, image: &mut ImageStore) -> Option<Box<dyn Command>> {
        if let Some(rect) = self.dirty_rect {
            let x = rect.min.x as u32;
            let y = rect.min.y as u32;
            let w = (rect.width() as u32 + 1).min(image.width().saturating_sub(x));
            let h = (rect.height() as u32 + 1).min(image.height().saturating_sub(y));
            let layer_index = image.active_layer;
            if let Some(layer) = image.layers.get_mut(layer_index) {
                let target = &mut layer.pixels;
                {
                    if w > 0 && h > 0 {
                        use image::GenericImageView;
                        let old_patch = target.view(x, y, w, h).to_image();
                        let src = self.layer.view(x, y, w, h).to_image();
                        for py in 0..h { for px in 0..w {
                            let p = src.get_pixel(px, py);
                            if p[3] > 0 { target.put_pixel(x+px, y+py, *p); }
                        }}
                        let new_patch = target.view(x, y, w, h).to_image();
                        image.mark_dirty();
                        self.dirty_rect = None;
                        self.phase = CurvePhase::Idle;
                        return Some(Box::new(PatchCommand { name: "Curve".to_string(), layer_index, x, y, old_patch, new_patch }));
                    }
                }
            }
        }
        self.phase = CurvePhase::Idle;
        None
    }
}

impl Tool for CurveTool {
    fn name(&self) -> &str { "Curve" }

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
        self.line_width = settings.stroke_size.px() / 2.0;
        self.color = color;

        match self.phase {
            CurvePhase::Idle => {
                if input.is_pressed {
                    if let Some(pos) = input.pos { self.p0 = pos; self.p1 = pos; self.p2 = pos; self.p3 = pos; }
                    self.phase = CurvePhase::DrawingLine;
                }
            }
            CurvePhase::DrawingLine => {
                if let Some(pos) = input.pos {
                    self.p3 = pos;
                    self.p1 = self.p0.lerp(self.p3, 0.33);
                    self.p2 = self.p0.lerp(self.p3, 0.66);
                    self.redraw_layer();
                }
                if input.is_released { self.phase = CurvePhase::BendingFirst; }
            }
            CurvePhase::BendingFirst => {
                if input.is_pressed { if let Some(pos) = input.pos { self.p1 = pos; self.p2 = pos; self.redraw_layer(); } }
                if input.is_released { self.phase = CurvePhase::BendingSecond; }
            }
            CurvePhase::BendingSecond => {
                if input.is_pressed { if let Some(pos) = input.pos { self.p2 = pos; self.redraw_layer(); } }
                if input.is_released { return self.commit(image); }
            }
        }
        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        self.dirty_rect.map(|_| (&self.layer, 0, 0))
    }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _settings: &crate::state::ToolSettings, pos: Pos2) {
        painter.circle_filled(pos, 3.0, Color32::WHITE);
    }

    fn configure(&mut self, _ui: &mut Ui, _settings: &mut crate::state::ToolSettings) {}
}
