use egui::{Color32, Context};

// ArsPaint dark palette (Tokyo-Night-ish), promoted from the old inline values.
pub const PANEL_FILL: Color32 = Color32::from_rgb(26, 27, 38);   // #1A1B26
pub const EXTREME_BG: Color32 = Color32::from_rgb(22, 22, 30);   // #16161E
pub const CANVAS_VOID: Color32 = Color32::from_rgb(14, 14, 22);  // #0E0E16
pub const ACCENT: Color32 = Color32::from_rgb(122, 162, 247);    // #7AA2F7
pub const ACTIVE_HL: Color32 = Color32::from_rgb(40, 52, 87);    // #283457
pub const SEP: Color32 = Color32::from_rgb(35, 38, 58);          // #23263A
pub const TEXT: Color32 = Color32::from_rgb(192, 202, 245);      // #C0CAF5
pub const MUTED: Color32 = Color32::from_rgb(169, 177, 214);     // #A9B1D6
pub const FAINT: Color32 = Color32::from_rgb(86, 95, 137);       // #565F89

pub fn apply(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = PANEL_FILL;
    visuals.panel_fill = PANEL_FILL;
    visuals.extreme_bg_color = EXTREME_BG;
    ctx.set_visuals(visuals);
}
