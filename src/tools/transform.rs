use crate::commands::{Command, PatchCommand};
use crate::image_store::ImageStore;
use crate::layers::LayerData;
use crate::state::ToolSettings;
use crate::tools::{Tool, ToolInput};
use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub struct TransformTool {
    floating_buffer: Option<RgbaImage>,
    source_rect: Option<Rect>,
    current_rect: Option<Rect>,
    is_dragging: bool,
    drag_start: Option<Pos2>,
    drag_offset: Vec2,
    handle_drag: Option<HandleType>,
    committed: bool,
    original_layer_snapshot: Option<RgbaImage>,
    layer_index: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum HandleType {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl TransformTool {
    pub fn new() -> Self {
        Self {
            floating_buffer: None,
            source_rect: None,
            current_rect: None,
            is_dragging: false,
            drag_start: None,
            drag_offset: Vec2::ZERO,
            handle_drag: None,
            committed: false,
            original_layer_snapshot: None,
            layer_index: 0,
        }
    }

    fn pick_up_selection(&mut self, image: &mut ImageStore) {
        if let Some(mask) = &image.selection {
            let mut min_x = mask.width();
            let mut max_x = 0;
            let mut min_y = mask.height();
            let mut max_y = 0;
            let mut found = false;

            for (x, y, p) in mask.enumerate_pixels() {
                if p[0] > 0 {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    found = true;
                }
            }

            if found {
                let w = max_x - min_x + 1;
                let h = max_y - min_y + 1;
                let mut buffer = ImageBuffer::new(w, h);

                self.layer_index = image.active_layer;
                let layer_img = match &mut image.layers[self.layer_index].data {
                    crate::layers::LayerData::Raster(img) => Some(img),
                    crate::layers::LayerData::Tone { buffer, .. } => Some(buffer),
                    _ => None,
                };

                if let Some(layer_img) = layer_img {
                    self.original_layer_snapshot = Some(layer_img.clone());

                    for y in 0..h {
                        for x in 0..w {
                            let cx = min_x + x;
                            let cy = min_y + y;
                            if mask.get_pixel(cx, cy)[0] > 0 {
                                buffer.put_pixel(x, y, *layer_img.get_pixel(cx, cy));
                                layer_img.put_pixel(cx, cy, Rgba([0, 0, 0, 0]));
                            }
                        }
                    }
                    image.mark_dirty();

                    let rect = Rect::from_min_max(
                        Pos2::new(min_x as f32, min_y as f32),
                        Pos2::new((max_x + 1) as f32, (max_y + 1) as f32),
                    );
                    self.floating_buffer = Some(buffer);
                    self.source_rect = Some(rect);
                    self.current_rect = Some(rect);
                }
            }
        }
    }
}

impl Tool for TransformTool {
    fn name(&self) -> &str {
        "Transform"
    }

    fn update(
        &mut self,
        image: &mut ImageStore,
        _settings: &ToolSettings,
        input: &ToolInput,
        _color: Rgba<u8>,
    ) -> Option<Box<dyn Command>> {
        if self.committed {
            if let (Some(buffer), Some(current), Some(old_snapshot)) = (
                &self.floating_buffer,
                self.current_rect,
                &self.original_layer_snapshot,
            ) {
                let layer_index = self.layer_index;
                let (w, h) = (image.width(), image.height());

                let target_buffer = match &mut image.layers[layer_index].data {
                    crate::layers::LayerData::Raster(img) => Some(img),
                    crate::layers::LayerData::Tone { buffer, .. } => Some(buffer),
                    _ => None,
                };

                if let Some(target_buffer) = target_buffer {
                    let nw = current.width().max(1.0) as u32;
                    let nh = current.height().max(1.0) as u32;

                    let resized = image::imageops::resize(
                        buffer,
                        nw,
                        nh,
                        image::imageops::FilterType::Nearest,
                    );

                    let tx = current.min.x as i32;
                    let ty = current.min.y as i32;

                    for y in 0..nh {
                        for x in 0..nw {
                            let cx = tx + x as i32;
                            let cy = ty + y as i32;

                            if cx >= 0 && cx < w as i32 && cy >= 0 && cy < h as i32 {
                                let p = resized.get_pixel(x, y);
                                if p[3] > 0 {
                                    target_buffer.put_pixel(cx as u32, cy as u32, *p);
                                }
                            }
                        }
                    }

                    let new_snapshot = target_buffer.clone();
                    image.mark_dirty();
                    self.committed = false;
                    self.floating_buffer = None;

                    return Some(Box::new(PatchCommand {
                        name: "Transform".to_string(),
                        layer_index,
                        x: 0,
                        y: 0,
                        old_patch: old_snapshot.clone(),
                        new_patch: new_snapshot,
                    }));
                }
            }
        }

        if self.floating_buffer.is_none() && !self.committed {
            self.pick_up_selection(image);
        }

        if let Some(mut current) = self.current_rect {
            if input.is_pressed {
                if let Some(mouse_pos) = input.pos {
                    if !self.is_dragging {
                        let handle_size = 12.0;
                        if mouse_pos.distance(current.left_top()) < handle_size {
                            self.handle_drag = Some(HandleType::TopLeft);
                        } else if mouse_pos.distance(current.right_top()) < handle_size {
                            self.handle_drag = Some(HandleType::TopRight);
                        } else if mouse_pos.distance(current.left_bottom()) < handle_size {
                            self.handle_drag = Some(HandleType::BottomLeft);
                        } else if mouse_pos.distance(current.right_bottom()) < handle_size {
                            self.handle_drag = Some(HandleType::BottomRight);
                        } else if current.contains(mouse_pos) {
                            self.handle_drag = Some(HandleType::Center);
                            self.drag_offset = mouse_pos - current.min;
                        }

                        if self.handle_drag.is_some() {
                            self.is_dragging = true;
                            self.drag_start = Some(mouse_pos);
                        }
                    } else {
                        match self.handle_drag {
                            Some(HandleType::Center) => {
                                let new_min = mouse_pos - self.drag_offset;
                                let size = current.size();
                                current = Rect::from_min_size(new_min, size);
                            }
                            Some(HandleType::TopLeft) => {
                                current.min = mouse_pos;
                            }
                            Some(HandleType::TopRight) => {
                                current.max.x = mouse_pos.x;
                                current.min.y = mouse_pos.y;
                            }
                            Some(HandleType::BottomLeft) => {
                                current.min.x = mouse_pos.x;
                                current.max.y = mouse_pos.y;
                            }
                            Some(HandleType::BottomRight) => {
                                current.max = mouse_pos;
                            }
                            _ => {}
                        }
                        self.current_rect = Some(current);
                    }
                }
            } else {
                self.is_dragging = false;
                self.handle_drag = None;
            }
        }

        None
    }

    fn get_temp_layer(&self) -> Option<(&RgbaImage, u32, u32)> {
        None
    }

    fn draw_cursor(&self, _ui: &mut Ui, painter: &Painter, _settings: &ToolSettings, _pos: Pos2) {
        if let Some(current) = self.current_rect {
            painter.rect_stroke(current, 0.0, egui::Stroke::new(1.0, Color32::WHITE));
            let handle_color = Color32::WHITE;
            painter.circle_filled(current.left_top(), 4.0, handle_color);
            painter.circle_filled(current.right_top(), 4.0, handle_color);
            painter.circle_filled(current.left_bottom(), 4.0, handle_color);
            painter.circle_filled(current.right_bottom(), 4.0, handle_color);
        }
    }

    fn configure(&mut self, ui: &mut Ui, _settings: &mut ToolSettings) {
        ui.vertical(|ui| {
            if self.floating_buffer.is_some() {
                ui.label("Transforming selection...");
                if ui.button("Confirm").clicked() {
                    self.committed = true;
                }
            } else {
                ui.label("Select an area first.");
            }
        });
    }
}
