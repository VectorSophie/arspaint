use crate::commands::Command;
use crate::image_store::ImageStore;
use crate::tools::base::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Ui};
use image::{GenericImageView, Rgba, RgbaImage};

pub struct EyedropperTool {
    pub picked: Option<(Rgba<u8>, bool)>, // (color, is_right_click)
}

impl EyedropperTool {
    pub fn new() -> Self { Self { picked: None } }
}

impl Tool for EyedropperTool {
    fn name(&self) -> &str { "Color Picker" }

    fn update(
        &mut self,
        image: &mut ImageStore,
        _settings: &crate::state::ToolSettings,
        input: &ToolInput,
        _color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        self.picked = None;
        if input.is_pressed || input.is_released {
            if let Some(pos) = input.pos {
                let x = pos.x as i32;
                let y = pos.y as i32;
                if x >= 0 && y >= 0 {
                    let (x, y) = (x as u32, y as u32);
                    if x < image.width() && y < image.height() {
                        let composite = image.get_composite();
                        let picked = *composite.get_pixel(x, y);
                        // is_right_click encoded via color == secondary — caller checks input
                        self.picked = Some((picked, false));
                    }
                }
            }
        }
        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> { None }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _settings: &crate::state::ToolSettings, pos: Pos2) {
        painter.circle_stroke(pos, 6.0, egui::Stroke::new(1.5, Color32::WHITE));
        painter.circle_stroke(pos, 6.5, egui::Stroke::new(0.5, Color32::BLACK));
    }

    fn configure(&mut self, _ui: &mut Ui, _settings: &mut crate::state::ToolSettings) {}
}
