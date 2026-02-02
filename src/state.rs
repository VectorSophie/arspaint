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

pub struct AppState {
    pub image: ImageStore,
    pub command_stack: CommandStack,
    pub active_tool: Box<dyn Tool>,
    pub tool_settings: ToolSettings,
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
            primary_color: Rgba([0, 0, 0, 255]),
            secondary_color: Rgba([255, 255, 255, 255]),
            palette,
        }
    }
}
