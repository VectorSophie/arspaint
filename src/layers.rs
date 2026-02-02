use egui::{Pos2, Rect};
use image::{ImageBuffer, Rgba, RgbaImage};
// use serde::{Deserialize, Serialize}; // Optional, but good practice

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BlendMode {
    Normal,
    Multiply,
    Add,
    Screen,
}

#[derive(Clone, Debug)]
pub enum VectorShape {
    Line {
        start: Pos2,
        end: Pos2,
        color: Rgba<u8>,
        width: f32,
    },
    Rectangle {
        rect: Rect,
        color: Rgba<u8>,
        width: f32,
        fill: bool,
    },
    Ellipse {
        rect: Rect,
        color: Rgba<u8>,
        width: f32,
        fill: bool,
    },
}

#[derive(Clone)]
pub enum LayerData {
    Raster(RgbaImage),
    Vector(Vec<VectorShape>),
    // Tone layers are essentially raster layers with a procedural effect applied during composite
    Tone {
        buffer: RgbaImage,
        frequency: f32, // Dots per unit
        density: f32,   // 0-1
    },
}

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub alpha_locked: bool,
    pub clipped: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    pub data: LayerData,
}

impl Layer {
    pub fn new_raster(width: u32, height: u32, name: String) -> Self {
        Self {
            name,
            visible: true,
            locked: false,
            alpha_locked: false,
            clipped: false,
            opacity: 1.0,
            blend: BlendMode::Normal,
            data: LayerData::Raster(ImageBuffer::new(width, height)),
        }
    }

    pub fn new_vector(name: String) -> Self {
        Self {
            name,
            visible: true,
            locked: false,
            alpha_locked: false,
            clipped: false,
            opacity: 1.0,
            blend: BlendMode::Normal,
            data: LayerData::Vector(Vec::new()),
        }
    }
}
