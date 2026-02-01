use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

#[derive(Clone)]
pub struct ImageStore {
    pub buffer: RgbaImage,
}

impl ImageStore {
    pub fn new(width: u32, height: u32) -> Self {
        // Initialize with white background
        let buffer = ImageBuffer::from_pixel(width, height, Rgba([255, 255, 255, 255]));
        Self { buffer }
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let img = image::open(path).context("Failed to open image file")?;
        let buffer = img.to_rgba8();
        Ok(Self { buffer })
    }

    pub fn width(&self) -> u32 {
        self.buffer.width()
    }

    pub fn height(&self) -> u32 {
        self.buffer.height()
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Rgba<u8>> {
        if x < self.width() && y < self.height() {
            Some(*self.buffer.get_pixel(x, y))
        } else {
            None
        }
    }

    pub fn put_pixel(&mut self, x: u32, y: u32, color: Rgba<u8>) {
        if x < self.width() && y < self.height() {
            self.buffer.put_pixel(x, y, color);
        }
    }

    pub fn get_buffer(&self) -> &RgbaImage {
        &self.buffer
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.buffer.save(path).context("Failed to save image")?;
        Ok(())
    }
}
