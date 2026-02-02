use crate::commands::Command;
use crate::image_store::ImageStore;
use crate::state::ToolSettings;
use crate::tools::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui};
use image::{ImageBuffer, Luma, RgbaImage};

pub struct RectSelectionTool {
    start_pos: Option<Pos2>,
    current_pos: Option<Pos2>,
}

impl RectSelectionTool {
    pub fn new() -> Self {
        Self {
            start_pos: None,
            current_pos: None,
        }
    }
}

impl Tool for RectSelectionTool {
    fn name(&self) -> &str {
        "Rect Selection"
    }

    fn update(
        &mut self,
        image: &mut ImageStore,
        _settings: &ToolSettings,
        input: &ToolInput,
        _color: image::Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if input.is_pressed {
            if self.start_pos.is_none() {
                self.start_pos = input.pos;
            }
            self.current_pos = input.pos;
        }

        if input.is_released {
            if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
                let min_x = (start.x.min(end.x) as i32).max(0) as u32;
                let max_x = (start.x.max(end.x) as i32).max(0) as u32;
                let min_y = (start.y.min(end.y) as i32).max(0) as u32;
                let max_y = (start.y.max(end.y) as i32).max(0) as u32;

                let w = image.width();
                let h = image.height();
                let mut mask = ImageBuffer::new(w, h);

                for y in min_y..max_y.min(h) {
                    for x in min_x..max_x.min(w) {
                        mask.put_pixel(x, y, Luma([255]));
                    }
                }

                if max_x > min_x && max_y > min_y {
                    image.selection = Some(mask);
                } else {
                    image.selection = None;
                }
            }
            self.start_pos = None;
            self.current_pos = None;
        }

        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        None
    }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _settings: &ToolSettings, pos: Pos2) {
        painter.circle_filled(pos, 2.0, Color32::LIGHT_BLUE);
        if let (Some(start), Some(current)) = (self.start_pos, Some(pos)) {
            let rect = Rect::from_two_pos(start, current);
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::LIGHT_BLUE));
        }
    }

    fn configure(&mut self, ui: &mut Ui, _settings: &mut ToolSettings) {
        ui.label("Drag to select a rectangular area.");
    }
}

pub struct LassoSelectionTool {
    points: Vec<Pos2>,
}

impl LassoSelectionTool {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    fn is_inside(&self, p: Pos2) -> bool {
        let mut inside = false;
        let mut j = self.points.len() - 1;
        for i in 0..self.points.len() {
            if ((self.points[i].y > p.y) != (self.points[j].y > p.y))
                && (p.x
                    < (self.points[j].x - self.points[i].x) * (p.y - self.points[i].y)
                        / (self.points[j].y - self.points[i].y)
                        + self.points[i].x)
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    }
}

impl Tool for LassoSelectionTool {
    fn name(&self) -> &str {
        "Lasso Selection"
    }

    fn update(
        &mut self,
        image: &mut ImageStore,
        _settings: &ToolSettings,
        input: &ToolInput,
        _color: image::Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if input.is_pressed {
            if let Some(pos) = input.pos {
                self.points.push(pos);
            }
        }

        if input.is_released && !self.points.is_empty() {
            let w = image.width();
            let h = image.height();
            let mut mask = ImageBuffer::new(w, h);

            if self.points.len() > 2 {
                let mut min_x: f32 = w as f32;
                let mut max_x: f32 = 0.0;
                let mut min_y: f32 = h as f32;
                let mut max_y: f32 = 0.0;

                for p in &self.points {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }

                let start_x = (min_x as i32).max(0) as u32;
                let end_x = (max_x as i32).max(0) as u32;
                let start_y = (min_y as i32).max(0) as u32;
                let end_y = (max_y as i32).max(0) as u32;

                for y in start_y..end_y.min(h) {
                    for x in start_x..end_x.min(w) {
                        if self.is_inside(Pos2::new(x as f32, y as f32)) {
                            mask.put_pixel(x, y, Luma([255]));
                        }
                    }
                }
                image.selection = Some(mask);
            }

            self.points.clear();
        }

        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        None
    }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _settings: &ToolSettings, pos: Pos2) {
        painter.circle_filled(pos, 2.0, Color32::LIGHT_BLUE);
        if self.points.len() > 1 {
            for i in 0..self.points.len() - 1 {
                painter.line_segment(
                    [self.points[i], self.points[i + 1]],
                    egui::Stroke::new(1.0, Color32::LIGHT_BLUE),
                );
            }
            painter.line_segment(
                [*self.points.last().unwrap(), pos],
                egui::Stroke::new(1.0, Color32::LIGHT_BLUE),
            );
        }
    }

    fn configure(&mut self, ui: &mut Ui, _settings: &mut ToolSettings) {
        ui.label("Draw a free-form path to select an area.");
    }
}
