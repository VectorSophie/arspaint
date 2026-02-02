use crate::commands::CommandStack;
use crate::image_store::ImageStore;
use crate::tools::{BrushTool, Tool};
use image::Rgba;

pub struct ToolSettings {
    pub brush_size: f32,
    pub brush_stabilization: f32,
    pub brush_spacing: f32,
    pub eraser_size: f32,
    pub line_width: f32,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            brush_size: 5.0,
            brush_stabilization: 0.5,
            brush_spacing: 0.1,
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
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn ctrl(mut self, value: bool) -> Self {
        self.ctrl = value;
        self
    }

    pub fn shift(mut self, value: bool) -> Self {
        self.shift = value;
        self
    }

    pub fn alt(mut self, value: bool) -> Self {
        self.alt = value;
        self
    }

    pub fn matches(&self, i: &egui::InputState) -> bool {
        i.key_pressed(self.key)
            && i.modifiers.ctrl == self.ctrl
            && i.modifiers.shift == self.shift
            && i.modifiers.alt == self.alt
    }

    pub fn format(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        s.push_str(&format!("{:?}", self.key));
        s
    }
}

pub struct Keybindings {
    pub undo: Shortcut,
    pub redo: Shortcut,
    pub brush: Shortcut,
    pub eraser: Shortcut,
    pub line: Shortcut,
    pub rect: Shortcut,
    pub ellipse: Shortcut,
    pub select: Shortcut,
    pub deselect: Shortcut,
    pub transform: Shortcut,
    pub pan: egui::Key,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            undo: Shortcut::new(egui::Key::Z).ctrl(true),
            redo: Shortcut::new(egui::Key::Y).ctrl(true),
            brush: Shortcut::new(egui::Key::B),
            eraser: Shortcut::new(egui::Key::E),
            line: Shortcut::new(egui::Key::L),
            rect: Shortcut::new(egui::Key::R),
            ellipse: Shortcut::new(egui::Key::O),
            select: Shortcut::new(egui::Key::S),
            deselect: Shortcut::new(egui::Key::D).ctrl(true),
            transform: Shortcut::new(egui::Key::T).ctrl(true),
            pan: egui::Key::Space,
        }
    }
}

pub struct AppState {
    pub image: ImageStore,
    pub command_stack: CommandStack,
    pub active_tool: Box<dyn Tool>,
    pub tool_settings: ToolSettings,
    pub keybindings: Keybindings,
    pub primary_color: Rgba<u8>,
    pub secondary_color: Rgba<u8>,
    pub palette: Vec<Rgba<u8>>,
}

impl AppState {
    pub fn new(width: u32, height: u32) -> Self {
        let palette = vec![
            Rgba([0, 0, 0, 255]),
            Rgba([255, 255, 255, 255]),
            Rgba([255, 0, 0, 255]),
            Rgba([0, 255, 0, 255]),
            Rgba([0, 0, 255, 255]),
            Rgba([255, 255, 0, 255]),
            Rgba([255, 0, 255, 255]),
            Rgba([0, 255, 255, 255]),
        ];

        Self {
            image: ImageStore::new(width, height),
            command_stack: CommandStack::new(),
            active_tool: Box::new(BrushTool::new(width, height)),
            tool_settings: ToolSettings::default(),
            keybindings: Keybindings::default(),
            primary_color: Rgba([0, 0, 0, 255]),
            secondary_color: Rgba([255, 255, 255, 255]),
            palette,
        }
    }
}
