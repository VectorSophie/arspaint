use super::ArsApp;
use eframe::egui::{self, Ui};

impl ArsApp {
    pub(super) fn render_history_panel(&mut self, ui: &mut Ui) {
        let cursor = self.state.command_stack.cursor();
        let labels: Vec<String> = self.state.command_stack.entries().iter().map(|c| c.name().to_string()).collect();

        // "Initial state" entry = index 0
        let mut jump_target: Option<usize> = None;
        if ui.selectable_label(cursor == 0, "● Initial state").clicked() {
            jump_target = Some(0);
        }
        for (i, label) in labels.iter().enumerate() {
            let applied = i < cursor; // entries [0, cursor) are applied
            let is_current = i + 1 == cursor;
            let text = if is_current { format!("● {label}") } else { label.clone() };
            let rich = if applied { egui::RichText::new(text) } else { egui::RichText::new(text).weak() };
            if ui.selectable_label(is_current, rich).clicked() {
                jump_target = Some(i + 1);
            }
        }
        if let Some(t) = jump_target {
            self.state.command_stack.jump_to(t, &mut self.state.image);
            self.image_dirty = true;
        }
    }
}
