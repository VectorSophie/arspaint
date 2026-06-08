use super::ArsApp;
use egui::{Color32, PointerButton, Pos2, Rect, Sense, Ui, Vec2};
use crate::tools::{AirbrushTool, BrushTool, EyedropperTool, FillTool, RectSelectionTool, SmearTool, ToolInput};

/// Map a screen-space point to image pixel coords (may be outside the image).
/// `image_min` is the top-left of the drawn image rect; `zoom` is px-per-image-px.
pub fn screen_to_image_px(screen: Pos2, image_min: Pos2, zoom: f32) -> Pos2 {
    let rel = screen - image_min;
    Pos2::new(rel.x / zoom, rel.y / zoom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_origin() {
        let p = screen_to_image_px(Pos2::new(100.0, 50.0), Pos2::new(100.0, 50.0), 1.0);
        assert_eq!(p, Pos2::new(0.0, 0.0));
    }

    #[test]
    fn divides_by_zoom() {
        let p = screen_to_image_px(Pos2::new(140.0, 90.0), Pos2::new(100.0, 50.0), 4.0);
        assert_eq!(p, Pos2::new(10.0, 10.0));
    }
}

impl ArsApp {
    // ── Canvas ────────────────────────────────────────────────────────────────

    pub fn render_canvas(&mut self, ui: &mut Ui) {
        let canvas_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(canvas_size, Sense::drag());

        let image_size = Vec2::new(
            self.state.image.width() as f32,
            self.state.image.height() as f32,
        ) * self.zoom;

        let screen_center = response.rect.center();
        let image_rect = Rect::from_center_size(screen_center + self.pan, image_size);

        // Checkerboard background
        let checker_size = 16.0 * self.zoom;
        let checker_painter = painter.with_clip_rect(image_rect);
        checker_painter.rect_filled(image_rect, 0.0, Color32::from_gray(200));
        let rows = (image_rect.height() / checker_size).ceil() as i32;
        let cols = (image_rect.width() / checker_size).ceil() as i32;
        for r in 0..rows {
            for c in 0..cols {
                if (r + c) % 2 == 1 {
                    let rect = Rect::from_min_size(
                        image_rect.min + Vec2::new(c as f32 * checker_size, r as f32 * checker_size),
                        Vec2::splat(checker_size),
                    );
                    checker_painter.rect_filled(rect.intersect(image_rect), 0.0, Color32::from_gray(180));
                }
            }
        }

        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        if let Some(tex) = &self.base_texture {
            painter.image(tex.id(), image_rect, uv, Color32::WHITE);
        }

        // Draw floating selection at its canvas position
        if let Some(fs) = &self.state.floating_selection {
            if let Some(tex) = &self.layer_texture {
                let fw = fs.image.width() as f32 * self.zoom;
                let fh = fs.image.height() as f32 * self.zoom;
                let fx = image_rect.min.x + fs.pos.x * self.zoom;
                let fy = image_rect.min.y + fs.pos.y * self.zoom;
                let fs_rect = Rect::from_min_size(Pos2::new(fx, fy), Vec2::new(fw, fh));
                painter.image(tex.id(), fs_rect, uv, Color32::WHITE);
                // dashed border around floating selection
                painter.rect_stroke(fs_rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 180, 255, 200)));
            }
        } else if let Some(tex) = &self.layer_texture {
            // tool temp layer (brush stroke in progress etc.)
            painter.image(tex.id(), image_rect, uv, Color32::WHITE);
        }

        if let Some(tex) = &self.selection_texture {
            painter.image(tex.id(), image_rect, uv, Color32::WHITE);
        }

        // Grid at high zoom
        if self.show_grid && self.zoom >= 4.0 {
            for gx in 0..=self.state.image.width() {
                let sx = image_rect.min.x + gx as f32 * self.zoom;
                painter.line_segment(
                    [Pos2::new(sx, image_rect.min.y), Pos2::new(sx, image_rect.max.y)],
                    egui::Stroke::new(0.3, Color32::from_rgba_unmultiplied(0,0,0,60)),
                );
            }
            for gy in 0..=self.state.image.height() {
                let sy = image_rect.min.y + gy as f32 * self.zoom;
                painter.line_segment(
                    [Pos2::new(image_rect.min.x, sy), Pos2::new(image_rect.max.x, sy)],
                    egui::Stroke::new(0.3, Color32::from_rgba_unmultiplied(0,0,0,60)),
                );
            }
        }

        // Canvas border
        painter.rect_stroke(image_rect, 0.0, egui::Stroke::new(1.0, Color32::from_gray(60)));

        // Snapshot keybinding values (Copy types) to avoid holding a borrow across closures
        let (kb_undo, kb_redo, kb_sel_all, kb_copy, kb_cut, kb_paste,
             kb_new, kb_open, kb_save, kb_save_as, kb_resize, kb_invert,
             kb_zoom_in, kb_zoom_out, kb_pan,
             kb_pencil, kb_brush, kb_eraser, kb_fill, kb_eye, kb_airbrush, kb_smear, kb_select) = {
            let kb = &self.state.keybindings;
            (kb.undo, kb.redo, kb.select_all, kb.copy, kb.cut, kb.paste,
             kb.new_file, kb.open_file, kb.save, kb.save_as, kb.resize_dialog, kb.invert,
             kb.zoom_in, kb.zoom_out, kb.pan,
             kb.pencil, kb.brush, kb.eraser, kb.fill, kb.eyedropper, kb.airbrush, kb.smear, kb.select)
        };

        let ctrl = ui.input(|i| i.modifiers.ctrl);
        let alt  = ui.input(|i| i.modifiers.alt);

        // Zoom via ctrl+scroll
        if ctrl {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom = (self.zoom * if scroll > 0.0 { 1.1 } else { 0.9 }).clamp(0.1, 50.0);
            }
        }

        // Alt = temporary eyedropper
        if alt && !self.alt_eyedropper_active {
            self.pre_alt_tool_name = self.state.active_tool.name().to_string();
            self.state.active_tool = Box::new(EyedropperTool::new());
            self.alt_eyedropper_active = true;
        } else if !alt && self.alt_eyedropper_active {
            self.alt_eyedropper_active = false;
            self.restore_pre_alt_tool();
        }

        // Pan
        let is_panning = response.dragged_by(PointerButton::Middle)
            || (ui.input(|i| i.key_down(kb_pan)) && response.dragged());
        if is_panning {
            self.pan += response.drag_delta();
        }

        if !is_panning {
            // Keyboard shortcuts — collect needed flags first, apply after
            let (do_undo, do_redo, do_sel_all, do_copy, do_cut, do_paste,
                 do_new, do_open, do_save, do_save_as, do_resize, do_invert,
                 do_zoom_in, do_zoom_out,
                 do_pencil, do_brush, do_eraser, do_fill, do_eye, do_airbrush, do_smear, do_select,
                 do_escape, do_delete) =
            ui.input(|i| (
                kb_undo.matches(i), kb_redo.matches(i), kb_sel_all.matches(i),
                kb_copy.matches(i), kb_cut.matches(i), kb_paste.matches(i),
                kb_new.matches(i), kb_open.matches(i), kb_save.matches(i), kb_save_as.matches(i),
                kb_resize.matches(i), kb_invert.matches(i),
                kb_zoom_in.matches(i), kb_zoom_out.matches(i),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_pencil)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_brush)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_eraser)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_fill)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_eye)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_airbrush)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_smear)),
                (!i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && i.key_pressed(kb_select)),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Delete),
            ));

            let img_w = self.state.image.width();
            let img_h = self.state.image.height();

            if do_undo  { self.state.command_stack.undo(&mut self.state.image); self.image_dirty = true; }
            if do_redo  { self.state.command_stack.redo(&mut self.state.image); self.image_dirty = true; }
            if do_sel_all { self.do_select_all(); }
            if do_copy  { self.do_copy(); }
            if do_cut   { self.do_cut(); }
            if do_paste { self.do_paste(); }
            if do_new   { self.show_new_dialog = true; }
            if do_open  { self.do_open(); }
            if do_save  { self.do_save(); }
            if do_save_as { self.do_save_as(); }
            if do_resize { self.resize_w = img_w; self.resize_h = img_h; self.show_resize_dialog = true; }
            if do_invert { self.state.image.invert_colors(); self.image_dirty = true; }
            if do_zoom_in  { self.zoom = (self.zoom * 1.25).min(50.0); }
            if do_zoom_out { self.zoom = (self.zoom / 1.25).max(0.1); }
            if do_pencil  { self.set_tool_pencil(); }
            if do_brush   { self.state.active_tool = Box::new(BrushTool::new(img_w, img_h)); }
            if do_eraser  { self.set_tool_eraser(); }
            if do_fill    { self.state.active_tool = Box::new(FillTool::new()); }
            if do_eye     { self.state.active_tool = Box::new(EyedropperTool::new()); }
            if do_airbrush{ self.state.active_tool = Box::new(AirbrushTool::new(img_w, img_h)); }
            if do_smear   { self.state.active_tool = Box::new(SmearTool::new(img_w, img_h)); }
            if do_select  { self.state.active_tool = Box::new(RectSelectionTool::new()); }
            if do_escape  {
                self.stamp_floating_selection();
                self.state.image.selection = None;
            }
            if do_delete  { self.do_delete_selection(); }

            // Tool update
            let pointer_pos = response.interact_pointer_pos();
            let hover_pos_in_image = pointer_pos.map(|pos| screen_to_image_px(pos, image_rect.min, self.zoom));

            // Track cursor for status bar
            if let Some(p) = response.hover_pos() {
                self.cursor_pos = Some(screen_to_image_px(p, image_rect.min, self.zoom));
            }

            let is_right = response.dragged_by(PointerButton::Secondary)
                || response.drag_started_by(PointerButton::Secondary);

            let input = ToolInput {
                pos: hover_pos_in_image,
                is_pressed: response.dragged_by(PointerButton::Primary)
                    || response.drag_started_by(PointerButton::Primary)
                    || is_right,
                is_released: response.drag_stopped_by(PointerButton::Primary)
                    || response.drag_stopped_by(PointerButton::Secondary),
            };

            let draw_color = if is_right { self.state.color2 } else { self.state.color1 };

            // Floating selection drag takes priority over tool
            let has_float = self.state.floating_selection.is_some();
            if has_float && input.is_pressed {
                if let Some(pos) = input.pos {
                    if let Some(fs) = &mut self.state.floating_selection {
                        let fs_rect = Rect::from_min_size(fs.pos, Vec2::new(
                            fs.image.width() as f32, fs.image.height() as f32,
                        ));
                        if fs_rect.contains(pos) {
                            let drag = response.drag_delta() / self.zoom;
                            fs.pos += drag;
                        }
                    }
                }
                if input.is_released { self.stamp_floating_selection(); }
            }

            let cmd = if !has_float {
                let c = self.state.active_tool.update(
                    &mut self.state.image,
                    &self.state.tool_settings,
                    &input,
                    draw_color,
                );
                // Eyedropper: apply picked color
                if let Some(picked) = self.state.active_tool.picked_color() {
                    if is_right { self.state.color2 = picked; } else { self.state.color1 = picked; }
                }
                c
            } else { None };

            if let Some(c) = cmd {
                self.state.command_stack.push(c);
                self.image_dirty = true;
            }

            if let Some(pos) = pointer_pos {
                if image_rect.contains(pos) {
                    self.state.active_tool.draw_cursor(ui, &painter, &self.state.tool_settings, pos);
                }
            }
        }
    }
}
