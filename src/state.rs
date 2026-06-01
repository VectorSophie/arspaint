use crate::commands::CommandStack;
use crate::image_store::ImageStore;
use crate::tools::{BrushTool, Tool};
use egui::Pos2;
use image::{RgbaImage, Rgba};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FillMode {
    Outline,
    Fill,
    Both,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StrokeSize {
    Thin,
    Medium,
    Thick,
    ExtraThick,
}

impl StrokeSize {
    pub fn px(self) -> f32 {
        match self {
            StrokeSize::Thin => 1.0,
            StrokeSize::Medium => 3.0,
            StrokeSize::Thick => 5.0,
            StrokeSize::ExtraThick => 8.0,
        }
    }
}

pub struct FloatingSelection {
    pub image: RgbaImage,
    pub pos: Pos2,
}

pub struct ToolSettings {
    pub brush_size: f32,
    pub brush_stabilization: f32,
    pub brush_spacing: f32,
    pub brush_opacity: f32,   // 0.0–1.0, Phase D
    pub airbrush_radius: f32,
    pub text_font_size: f32,
    pub text_bold: bool,
    pub text_italic: bool,
    pub shape_fill_mode: FillMode,
    pub stroke_size: StrokeSize,
    // kept for BrushTool/EraserTool compat
    pub eraser_size: f32,
    pub line_width: f32,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            brush_size: 5.0,
            brush_stabilization: 0.5,
            brush_spacing: 0.1,
            brush_opacity: 1.0,
            airbrush_radius: 20.0,
            text_font_size: 16.0,
            text_bold: false,
            text_italic: false,
            shape_fill_mode: FillMode::Outline,
            stroke_size: StrokeSize::Medium,
            eraser_size: 10.0,
            line_width: 2.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shortcut {
    pub key: egui::Key,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Shortcut {
    pub fn new(key: egui::Key) -> Self {
        Self { key, ctrl: false, shift: false, alt: false }
    }
    pub fn ctrl(mut self, value: bool) -> Self { self.ctrl = value; self }
    pub fn shift(mut self, value: bool) -> Self { self.shift = value; self }
    pub fn alt(mut self, value: bool) -> Self { self.alt = value; self }

    pub fn matches(&self, i: &egui::InputState) -> bool {
        i.key_pressed(self.key)
            && i.modifiers.ctrl == self.ctrl
            && i.modifiers.shift == self.shift
            && i.modifiers.alt == self.alt
    }

    pub fn format(&self) -> String {
        let mut s = String::new();
        if self.ctrl { s.push_str("Ctrl+"); }
        if self.shift { s.push_str("Shift+"); }
        if self.alt { s.push_str("Alt+"); }
        s.push_str(&format!("{:?}", self.key));
        s
    }
}

pub struct Keybindings {
    pub undo: Shortcut,
    pub redo: Shortcut,
    pub select_all: Shortcut,
    pub copy: Shortcut,
    pub cut: Shortcut,
    pub paste: Shortcut,
    pub new_file: Shortcut,
    pub open_file: Shortcut,
    pub save: Shortcut,
    pub save_as: Shortcut,
    pub resize_dialog: Shortcut,
    pub invert: Shortcut,
    pub pencil: egui::Key,
    pub brush: egui::Key,
    pub eraser: egui::Key,
    pub fill: egui::Key,
    pub eyedropper: egui::Key,
    pub text: egui::Key,
    pub magnifier: egui::Key,
    pub airbrush: egui::Key,
    pub select: egui::Key,
    pub zoom_in: Shortcut,
    pub zoom_out: Shortcut,
    pub pan: egui::Key,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            undo:          Shortcut::new(egui::Key::Z).ctrl(true),
            redo:          Shortcut::new(egui::Key::Y).ctrl(true),
            select_all:    Shortcut::new(egui::Key::A).ctrl(true),
            copy:          Shortcut::new(egui::Key::C).ctrl(true),
            cut:           Shortcut::new(egui::Key::X).ctrl(true),
            paste:         Shortcut::new(egui::Key::V).ctrl(true),
            new_file:      Shortcut::new(egui::Key::N).ctrl(true),
            open_file:     Shortcut::new(egui::Key::O).ctrl(true),
            save:          Shortcut::new(egui::Key::S).ctrl(true),
            save_as:       Shortcut::new(egui::Key::S).ctrl(true).shift(true),
            resize_dialog: Shortcut::new(egui::Key::E).ctrl(true),
            invert:        Shortcut::new(egui::Key::I).ctrl(true).shift(true),
            pencil:     egui::Key::P,
            brush:      egui::Key::B,
            eraser:     egui::Key::E,
            fill:       egui::Key::F,
            eyedropper: egui::Key::I,
            text:       egui::Key::T,
            magnifier:  egui::Key::M,
            airbrush:   egui::Key::A,
            select:     egui::Key::S,
            zoom_in:    Shortcut::new(egui::Key::Equals).ctrl(true),
            zoom_out:   Shortcut::new(egui::Key::Minus).ctrl(true),
            pan:        egui::Key::Space,
        }
    }
}

// Modern Paint 20-color palette (2 rows of 10)
pub const PALETTE: [Rgba<u8>; 20] = [
    Rgba([0,   0,   0,   255]), Rgba([127, 127, 127, 255]),
    Rgba([136, 0,   21,  255]), Rgba([237, 28,  36,  255]),
    Rgba([255, 127, 39,  255]), Rgba([255, 242, 0,   255]),
    Rgba([34,  177, 76,  255]), Rgba([0,   162, 232, 255]),
    Rgba([63,  72,  204, 255]), Rgba([163, 73,  164, 255]),
    Rgba([255, 255, 255, 255]), Rgba([195, 195, 195, 255]),
    Rgba([185, 122, 87,  255]), Rgba([255, 174, 201, 255]),
    Rgba([255, 201, 14,  255]), Rgba([239, 228, 176, 255]),
    Rgba([181, 230, 29,  255]), Rgba([153, 217, 234, 255]),
    Rgba([112, 146, 190, 255]), Rgba([200, 191, 231, 255]),
];

pub struct AppState {
    pub image: ImageStore,
    pub command_stack: CommandStack,
    pub active_tool: Box<dyn Tool>,
    pub tool_settings: ToolSettings,
    pub keybindings: Keybindings,
    pub color1: Rgba<u8>,
    pub color2: Rgba<u8>,
    pub clipboard: Option<RgbaImage>,
    pub floating_selection: Option<FloatingSelection>,
    pub current_save_path: Option<std::path::PathBuf>,
}

impl AppState {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            image: ImageStore::new(width, height),
            command_stack: CommandStack::new(),
            active_tool: Box::new(BrushTool::new(width, height)),
            tool_settings: ToolSettings::default(),
            keybindings: Keybindings::default(),
            color1: Rgba([0, 0, 0, 255]),
            color2: Rgba([255, 255, 255, 255]),
            clipboard: None,
            floating_selection: None,
            current_save_path: None,
        }
    }
}
