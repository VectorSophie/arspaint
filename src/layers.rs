use image::{ImageBuffer, RgbaImage};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BlendMode {
    Normal,
    Multiply,
    Add,
    Screen,
    Overlay,
    SoftLight,
    Difference,
}

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    pub locked: bool,
    pub alpha_locked: bool,
    pub clipped: bool,
    pub pixels: RgbaImage,
}

impl Layer {
    pub fn new_raster(width: u32, height: u32, name: String) -> Self {
        Self {
            name,
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            locked: false,
            alpha_locked: false,
            clipped: false,
            pixels: ImageBuffer::new(width, height),
        }
    }
}
