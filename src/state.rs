use crate::commands::CommandStack;
use crate::image_store::ImageStore;
use crate::tools::{BrushTool, Tool};
use image::Rgba;

pub struct AppState {
    pub image: ImageStore,
    pub command_stack: CommandStack,
    pub active_tool: Box<dyn Tool>,
    pub primary_color: Rgba<u8>,
    pub secondary_color: Rgba<u8>,
}

impl AppState {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            image: ImageStore::new(width, height),
            command_stack: CommandStack::new(),
            active_tool: Box::new(BrushTool::new(width, height)),
            primary_color: Rgba([0, 0, 0, 255]),
            secondary_color: Rgba([255, 255, 255, 255]),
        }
    }
}
