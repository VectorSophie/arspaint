use super::ArsApp;
use crate::tools::*;
use eframe::egui::{self, Ui};

impl ArsApp {
    pub(super) fn render_tools_panel(&mut self, ui: &mut Ui) {
        let (w, h) = (self.state.image.width(), self.state.image.height());
        let active = self.state.active_tool.name().to_string();

        // (icon, tool name as reported by Tool::name(), builder)
        // `None` builder == not-yet-implemented (greyed for faithful layout).
        type Build = Option<fn(u32, u32) -> Box<dyn Tool>>;
        let groups: [(&str, &[(&str, &str, Build)]); 5] = [
            ("Select", &[
                ("▱", "Rectangular Selection", Some(|_, _| Box::new(RectSelectionTool::new()) as Box<dyn Tool>)),
                ("◌", "Free-form Selection", Some(|_, _| Box::new(LassoSelectionTool::new()) as Box<dyn Tool>)),
                ("✦", "Magic Wand", None),
                ("✥", "Move", Some(|_, _| Box::new(TransformTool::new()) as Box<dyn Tool>)),
            ]),
            ("Draw", &[
                ("✏", "Pencil", Some(|w, h| Box::new(PencilTool::new(w, h)) as Box<dyn Tool>)),
                ("🖌", "Brush", Some(|w, h| Box::new(BrushTool::new(w, h)) as Box<dyn Tool>)),
                ("▰", "Eraser", Some(|w, h| Box::new(EraserTool::new(w, h)) as Box<dyn Tool>)),
                ("💨", "Airbrush", Some(|w, h| Box::new(AirbrushTool::new(w, h)) as Box<dyn Tool>)),
                ("🪣", "Fill", Some(|_, _| Box::new(FillTool::new()) as Box<dyn Tool>)),
                ("〰", "Smear", Some(|w, h| Box::new(SmearTool::new(w, h)) as Box<dyn Tool>)),
                ("◢", "Gradient", None),
            ]),
            ("Photo", &[
                ("💧", "Color Picker", Some(|_, _| Box::new(EyedropperTool::new()) as Box<dyn Tool>)),
                ("⎘", "Clone Stamp", None),
            ]),
            ("Shape", &[
                ("A", "Text", None),
                ("╱", "Line", Some(|w, h| Box::new(LineTool::new(w, h)) as Box<dyn Tool>)),
                ("〜", "Curve", Some(|w, h| Box::new(CurveTool::new(w, h)) as Box<dyn Tool>)),
                ("▭", "Rectangle", Some(|w, h| Box::new(RectangleTool::new(w, h)) as Box<dyn Tool>)),
                ("◯", "Ellipse", Some(|w, h| Box::new(EllipseTool::new(w, h)) as Box<dyn Tool>)),
            ]),
            ("More shapes", &[
                ("△", "Shape", Some(|w, h| Box::new(ShapeTool::new(ShapeKind::Triangle, w, h)) as Box<dyn Tool>)),
            ]),
        ];

        for (group, tools) in groups {
            ui.label(egui::RichText::new(group).small().color(super::theme::FAINT));
            egui::Grid::new(format!("toolgrid_{group}")).num_columns(2).spacing([4.0, 4.0]).show(ui, |ui| {
                for (i, (icon, name, build)) in tools.iter().enumerate() {
                    match build {
                        Some(f) => {
                            if ui.selectable_label(active == *name, *icon).on_hover_text(*name).clicked() {
                                self.state.active_tool = f(w, h);
                            }
                        }
                        None => { ui.add_enabled(false, egui::Button::new(*icon).frame(false)).on_hover_text(format!("{name} (coming soon)")); }
                    }
                    if i % 2 == 1 { ui.end_row(); }
                }
            });
            ui.separator();
        }
    }
}
