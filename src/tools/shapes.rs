use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::base::{Tool, ToolInput};
use crate::state::{FillMode, ToolSettings};
use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShapeKind {
    Triangle, RightTriangle, Diamond, Pentagon, Hexagon,
    Star4, Star6,
}

pub struct ShapeTool {
    pub kind: ShapeKind,
    layer: RgbaImage,
    start: Option<Pos2>,
    current: Option<Pos2>,
    dirty_rect: Option<Rect>,
}

impl ShapeTool {
    pub fn new(kind: ShapeKind, width: u32, height: u32) -> Self {
        Self {
            kind,
            layer: ImageBuffer::new(width, height),
            start: None,
            current: None,
            dirty_rect: None,
        }
    }

    // Generate polygon vertices for a shape given bounding box
    fn shape_points(kind: ShapeKind, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<[f32; 2]> {
        let cx = (x0 + x1) / 2.0;
        let cy = (y0 + y1) / 2.0;
        let rx = (x1 - x0).abs() / 2.0;
        let ry = (y1 - y0).abs() / 2.0;

        let reg_poly = |n: u32, rot: f32| -> Vec<[f32; 2]> {
            (0..n).map(|i| {
                let a = rot + i as f32 * std::f32::consts::TAU / n as f32;
                [cx + rx * a.cos(), cy + ry * a.sin()]
            }).collect()
        };

        let star = |n: u32, inner: f32| -> Vec<[f32; 2]> {
            let mut pts = Vec::new();
            for i in 0..n*2 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / n as f32;
                let r = if i % 2 == 0 { 1.0 } else { inner };
                pts.push([cx + rx * r * a.cos(), cy + ry * r * a.sin()]);
            }
            pts
        };

        match kind {
            ShapeKind::Triangle => vec![[cx, y0], [x1, y1], [x0, y1]],
            ShapeKind::RightTriangle => vec![[x0, y0], [x1, y1], [x0, y1]],
            ShapeKind::Diamond => vec![[cx, y0], [x1, cy], [cx, y1], [x0, cy]],
            ShapeKind::Pentagon  => reg_poly(5, -std::f32::consts::FRAC_PI_2),
            ShapeKind::Hexagon   => reg_poly(6, 0.0),
            ShapeKind::Star4 => star(4, 0.4),
            ShapeKind::Star6 => star(6, 0.5),
        }
    }

    fn rasterize_polygon(layer: &mut RgbaImage, pts: &[[f32;2]], color: Rgba<u8>, fill: FillMode, lw: f32, dirty: &mut Option<Rect>) {
        if pts.len() < 2 { return; }
        let w = layer.width() as i32;
        let h = layer.height() as i32;

        let put = |layer: &mut RgbaImage, x: i32, y: i32, dirty: &mut Option<Rect>| {
            if x >= 0 && x < w && y >= 0 && y < h {
                layer.put_pixel(x as u32, y as u32, color);
                let r = Rect::from_min_size(Pos2::new(x as f32, y as f32), Vec2::splat(1.0));
                *dirty = Some(match *dirty { Some(d) => d.union(r), None => r });
            }
        };

        // scanline fill
        if fill == FillMode::Fill || fill == FillMode::Both {
            let min_y = pts.iter().map(|p| p[1] as i32).min().unwrap_or(0).max(0);
            let max_y = pts.iter().map(|p| p[1] as i32).max().unwrap_or(0).min(h - 1);
            for scan_y in min_y..=max_y {
                let mut xs: Vec<f32> = Vec::new();
                let n = pts.len();
                for i in 0..n {
                    let (ax, ay) = (pts[i][0], pts[i][1]);
                    let (bx, by) = (pts[(i+1)%n][0], pts[(i+1)%n][1]);
                    let sy = scan_y as f32 + 0.5;
                    if (ay <= sy && by > sy) || (by <= sy && ay > sy) {
                        xs.push(ax + (sy - ay) / (by - ay) * (bx - ax));
                    }
                }
                xs.sort_by(|a,b| a.partial_cmp(b).unwrap());
                let mut i = 0;
                while i + 1 < xs.len() {
                    let x0 = xs[i] as i32;
                    let x1 = xs[i+1] as i32;
                    for x in x0..=x1 { put(layer, x, scan_y, dirty); }
                    i += 2;
                }
            }
        }

        // outline
        if fill == FillMode::Outline || fill == FillMode::Both {
            let n = pts.len();
            let r = lw as i32;
            let r_sq = r * r;
            for i in 0..n {
                let (ax, ay) = (pts[i][0], pts[i][1]);
                let (bx, by) = (pts[(i+1)%n][0], pts[(i+1)%n][1]);
                let dx = bx - ax; let dy = by - ay;
                let steps = (dx.abs() + dy.abs()) as u32 + 1;
                for s in 0..=steps {
                    let t = s as f32 / steps as f32;
                    let cx = (ax + dx * t) as i32;
                    let cy = (ay + dy * t) as i32;
                    for oy in -r..=r { for ox in -r..=r {
                        if ox*ox + oy*oy <= r_sq { put(layer, cx+ox, cy+oy, dirty); }
                    }}
                }
            }
        }
    }

    fn rasterize_ellipse(layer: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgba<u8>, fill: FillMode, lw: f32, dirty: &mut Option<Rect>) {
        let cx = (x0 + x1) / 2.0;
        let cy = (y0 + y1) / 2.0;
        let rx = ((x1 - x0).abs() / 2.0).max(0.5);
        let ry = ((y1 - y0).abs() / 2.0).max(0.5);
        let w = layer.width() as i32;
        let h = layer.height() as i32;

        let put = |layer: &mut RgbaImage, x: i32, y: i32, dirty: &mut Option<Rect>| {
            if x >= 0 && x < w && y >= 0 && y < h {
                layer.put_pixel(x as u32, y as u32, color);
                let r = Rect::from_min_size(Pos2::new(x as f32, y as f32), Vec2::splat(1.0));
                *dirty = Some(match *dirty { Some(d) => d.union(r), None => r });
            }
        };

        let min_x = (cx - rx - lw) as i32;
        let max_x = (cx + rx + lw) as i32;
        let min_y = (cy - ry - lw) as i32;
        let max_y = (cy + ry + lw) as i32;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = (px as f32 - cx) / rx;
                let dy = (py as f32 - cy) / ry;
                let d = dx*dx + dy*dy;
                let in_outer = d <= (1.0 + lw/rx.min(ry)).powi(2);
                let in_inner = d <= (1.0 - lw/rx.min(ry)).max(0.0).powi(2);

                match fill {
                    FillMode::Fill => { if d <= 1.0 { put(layer, px, py, dirty); } }
                    FillMode::Outline => { if in_outer && !in_inner { put(layer, px, py, dirty); } }
                    FillMode::Both => { if in_outer { put(layer, px, py, dirty); } }
                }
            }
        }
    }

    fn redraw(&mut self, settings: &ToolSettings, color: Rgba<u8>, shift: bool) {
        // clear
        if let Some(rect) = self.dirty_rect {
            let x = rect.min.x as u32; let y = rect.min.y as u32;
            let w = (rect.width() as u32 + 1).min(self.layer.width().saturating_sub(x));
            let h = (rect.height() as u32 + 1).min(self.layer.height().saturating_sub(y));
            for py in 0..h { for px in 0..w { self.layer.put_pixel(x+px, y+py, Rgba([0,0,0,0])); } }
        }
        self.dirty_rect = None;

        let (s, e) = match (self.start, self.current) { (Some(s), Some(e)) => (s, e), _ => return };
        let (mut x0, mut y0, mut x1, mut y1) = (s.x.min(e.x), s.y.min(e.y), s.x.max(e.x), s.y.max(e.y));
        if shift {
            let side = (x1-x0).min(y1-y0);
            x1 = x0 + side; y1 = y0 + side;
        }

        let lw = settings.stroke_size.px() / 2.0;
        let fill = settings.shape_fill_mode;

        let pts = Self::shape_points(self.kind, x0, y0, x1, y1);
        Self::rasterize_polygon(&mut self.layer, &pts, color, fill, lw, &mut self.dirty_rect);
    }

    fn commit(&mut self, image: &mut ImageStore) -> Option<Box<dyn Command>> {
        if let Some(rect) = self.dirty_rect {
            let x = rect.min.x as u32; let y = rect.min.y as u32;
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
                        self.start = None; self.current = None;
                        return Some(Box::new(PatchCommand {
                            name: format!("{:?}", self.kind),
                            layer_index, x, y, old_patch, new_patch,
                        }));
                    }
                }
            }
        }
        self.start = None; self.current = None;
        None
    }
}

impl Tool for ShapeTool {
    fn name(&self) -> &str { "Shape" }

    fn update(
        &mut self,
        image: &mut ImageStore,
        settings: &ToolSettings,
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if self.layer.width() != image.width() || self.layer.height() != image.height() {
            self.layer = ImageBuffer::new(image.width(), image.height());
        }

        let shift = false; // shift handled by caller via constrain
        if input.is_pressed {
            if self.start.is_none() { self.start = input.pos; }
            self.current = input.pos;
            self.redraw(settings, color, shift);
        } else if !input.is_pressed && !input.is_released {
            self.start = None; self.current = None;
        }
        if input.is_released { return self.commit(image); }
        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        self.dirty_rect.map(|_| (&self.layer, 0, 0))
    }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _s: &ToolSettings, pos: Pos2) {
        painter.circle_stroke(pos, 4.0, egui::Stroke::new(1.0, Color32::WHITE));
    }

    fn configure(&mut self, _ui: &mut Ui, _settings: &mut ToolSettings) {}

    fn active_shape_kind(&self) -> Option<ShapeKind> { Some(self.kind) }
}
