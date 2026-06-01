use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::tools::base::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Ui};
use image::{GenericImageView, Rgba, RgbaImage};
use std::collections::VecDeque;

pub struct FillTool;

impl FillTool {
    pub fn new() -> Self { Self }

    fn flood_fill(image: &mut RgbaImage, sx: u32, sy: u32, fill: Rgba<u8>, selection: Option<&image::GrayImage>) {
        let target = *image.get_pixel(sx, sy);
        if target == fill { return; }
        let w = image.width();
        let h = image.height();

        let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
        queue.push_back((sx, sy));

        while let Some((x, y)) = queue.pop_front() {
            if *image.get_pixel(x, y) != target { continue; }
            let in_sel = selection.map(|m| m.get_pixel(x, y)[0] > 0).unwrap_or(true);
            if !in_sel { continue; }
            image.put_pixel(x, y, fill);
            if x > 0     { queue.push_back((x - 1, y)); }
            if x + 1 < w { queue.push_back((x + 1, y)); }
            if y > 0     { queue.push_back((x, y - 1)); }
            if y + 1 < h { queue.push_back((x, y + 1)); }
        }
    }
}

impl Tool for FillTool {
    fn name(&self) -> &str { "Fill" }

    fn update(
        &mut self,
        image: &mut ImageStore,
        _settings: &crate::state::ToolSettings,
        input: &ToolInput,
        color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if input.is_released {
            if let Some(pos) = input.pos {
                let x = pos.x as i32;
                let y = pos.y as i32;
                if x < 0 || y < 0 { return None; }
                let (x, y) = (x as u32, y as u32);
                if x >= image.width() || y >= image.height() { return None; }

                let layer_index = image.active_layer;
                let old_snapshot = image.get_active_raster_snapshot()?;

                let selection_clone = image.selection.clone();
                if let Some(layer) = image.layers.get_mut(layer_index) {
                    if let crate::layers::LayerData::Raster(ref mut buf) | crate::layers::LayerData::Tone { buffer: ref mut buf, .. } = layer.data {
                        Self::flood_fill(buf, x, y, color, selection_clone.as_ref());
                        let new_snapshot = buf.clone();
                        image.mark_dirty();
                        return Some(Box::new(PatchCommand {
                            name: "Fill".to_string(),
                            layer_index,
                            x: 0, y: 0,
                            old_patch: old_snapshot,
                            new_patch: new_snapshot,
                        }));
                    }
                }
            }
        }
        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> { None }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _settings: &crate::state::ToolSettings, pos: Pos2) {
        painter.text(pos, egui::Align2::LEFT_TOP, "⬛", egui::FontId::proportional(16.0), Color32::WHITE);
    }

    fn configure(&mut self, _ui: &mut Ui, _settings: &mut crate::state::ToolSettings) {}
}
