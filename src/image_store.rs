use crate::layers::{BlendMode, Layer, LayerData};
use anyhow::{Context, Result};
use image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use std::path::Path;

#[derive(Clone)]
pub struct ImageStore {
    width: u32,
    height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    // Cached final render
    composite: RgbaImage,
    composite_dirty: bool,
}

impl ImageStore {
    pub fn new(width: u32, height: u32) -> Self {
        // Default to one raster layer
        let layer = Layer::new_raster(width, height, "Layer 1".to_string());
        // Initialize background to white
        let mut store = Self {
            width,
            height,
            layers: vec![layer],
            active_layer: 0,
            composite: ImageBuffer::new(width, height),
            composite_dirty: true,
        };

        // Fill first layer with white
        if let LayerData::Raster(ref mut img) = store.layers[0].data {
            for pixel in img.pixels_mut() {
                *pixel = Rgba([255, 255, 255, 255]);
            }
        }
        store.composite();
        store
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let img = image::open(path).context("Failed to open image file")?;
        let buffer = img.to_rgba8();
        let width = buffer.width();
        let height = buffer.height();

        let layer = Layer {
            name: "Background".to_string(),
            visible: true,
            locked: false,
            alpha_locked: false,
            clipped: false,
            opacity: 1.0,
            blend: BlendMode::Normal,
            data: LayerData::Raster(buffer),
        };

        let mut store = Self {
            width,
            height,
            layers: vec![layer],
            active_layer: 0,
            composite: ImageBuffer::new(width, height),
            composite_dirty: true,
        };
        store.composite();
        Ok(store)
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut Layer> {
        if self.active_layer < self.layers.len() {
            Some(&mut self.layers[self.active_layer])
        } else {
            None
        }
    }

    pub fn active_layer(&self) -> Option<&Layer> {
        self.layers.get(self.active_layer)
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.insert(self.active_layer + 1, layer);
        self.active_layer += 1;
        self.composite_dirty = true;
    }

    pub fn composite(&mut self) {
        if !self.composite_dirty {
            return;
        }

        // Clear composite
        self.composite =
            ImageBuffer::from_pixel(self.width, self.height, Rgba([200, 200, 200, 255])); // Checkerboard fallback?
                                                                                          // Actually, let's start transparent
        for p in self.composite.pixels_mut() {
            *p = Rgba([0, 0, 0, 0]);
        }

        let mut i = 0;
        while i < self.layers.len() {
            // Need to decoupling borrowing of self from layer
            // We can clone properties needed for blending
            let (visible, opacity, blend, data) = {
                let layer = &self.layers[i];
                (layer.visible, layer.opacity, layer.blend, &layer.data)
            };

            if visible {
                match data {
                    LayerData::Raster(img) => {
                        // Safe because we are not borrowing self.layers anymore, but we need img reference
                        // Uh oh, `data` borrows from `self.layers`.
                        // But `blend_buffer` borrows `self.composite` (mut).
                        // Rust won't like `self.layers` (immutable) and `self.composite` (part of self, mutable) being borrowed same time.
                        // We need to split ImageStore or use interior mutability or unsafe.
                        // OR: clone the buffer? Too slow.
                        // Better: Iterate over indices, and get references strictly.
                        // Actually, `composite` field is disjoint from `layers`.
                        // But the method takes `&mut self`.

                        // Workaround: Temporarily take composite out of self?
                        // Or pass `&mut composite` and `&layers` to a static function.
                    }
                    _ => {}
                }
            }
            i += 1;
        }

        // Let's refactor composite to be a static-like method that takes the dest and source layers
        // to avoid self-borrow conflicts.

        let dest = &mut self.composite;
        let layers = &self.layers;

        Self::composite_layers(dest, layers);
        self.composite_dirty = false;
    }

    fn composite_layers(dest: &mut RgbaImage, layers: &[Layer]) {
        for p in dest.pixels_mut() {
            *p = Rgba([0, 0, 0, 0]);
        }

        for i in 0..layers.len() {
            let layer = &layers[i];
            if !layer.visible {
                continue;
            }

            let mask = if layer.clipped && i > 0 {
                match &layers[i - 1].data {
                    LayerData::Raster(img) => Some(img),
                    LayerData::Tone { buffer, .. } => Some(buffer),
                    _ => None,
                }
            } else {
                None
            };

            match &layer.data {
                LayerData::Raster(img) => {
                    Self::blend_buffer_static(dest, img, layer.opacity, layer.blend, mask)
                }
                LayerData::Tone { buffer, .. } => {
                    Self::blend_buffer_static(dest, buffer, layer.opacity, layer.blend, mask)
                }
                _ => {}
            }
        }
    }

    fn blend_buffer_static(
        dest: &mut RgbaImage,
        source: &RgbaImage,
        opacity: f32,
        mode: BlendMode,
        mask: Option<&RgbaImage>,
    ) {
        for (x, y, pixel) in dest.enumerate_pixels_mut() {
            if x >= source.width() || y >= source.height() {
                continue;
            }
            let src_pixel = source.get_pixel(x, y);

            let mut src_a = (src_pixel[3] as f32 / 255.0) * opacity;

            if let Some(mask_img) = mask {
                if x < mask_img.width() && y < mask_img.height() {
                    let mask_pixel = mask_img.get_pixel(x, y);
                    src_a *= mask_pixel[3] as f32 / 255.0;
                } else {
                    src_a = 0.0;
                }
            }

            if src_a <= 0.0 {
                continue;
            }

            let dst_pixel = *pixel;
            let dst_a = dst_pixel[3] as f32 / 255.0;

            let (r, g, b) = match mode {
                BlendMode::Normal => (
                    src_pixel[0] as f32,
                    src_pixel[1] as f32,
                    src_pixel[2] as f32,
                ),
                BlendMode::Multiply => (
                    (dst_pixel[0] as f32 * src_pixel[0] as f32) / 255.0,
                    (dst_pixel[1] as f32 * src_pixel[1] as f32) / 255.0,
                    (dst_pixel[2] as f32 * src_pixel[2] as f32) / 255.0,
                ),
                BlendMode::Add => (
                    (dst_pixel[0] as f32 + src_pixel[0] as f32).min(255.0),
                    (dst_pixel[1] as f32 + src_pixel[1] as f32).min(255.0),
                    (dst_pixel[2] as f32 + src_pixel[2] as f32).min(255.0),
                ),
                BlendMode::Screen => {
                    let inv_src_r = 1.0 - (src_pixel[0] as f32 / 255.0);
                    let inv_dst_r = 1.0 - (dst_pixel[0] as f32 / 255.0);
                    let inv_src_g = 1.0 - (src_pixel[1] as f32 / 255.0);
                    let inv_dst_g = 1.0 - (dst_pixel[1] as f32 / 255.0);
                    let inv_src_b = 1.0 - (src_pixel[2] as f32 / 255.0);
                    let inv_dst_b = 1.0 - (dst_pixel[2] as f32 / 255.0);

                    (
                        (1.0 - (inv_src_r * inv_dst_r)) * 255.0,
                        (1.0 - (inv_src_g * inv_dst_g)) * 255.0,
                        (1.0 - (inv_src_b * inv_dst_b)) * 255.0,
                    )
                }
            };

            let out_a = src_a + dst_a * (1.0 - src_a);
            let out_r = (r * src_a + dst_pixel[0] as f32 * dst_a * (1.0 - src_a)) / out_a;
            let out_g = (g * src_a + dst_pixel[1] as f32 * dst_a * (1.0 - src_a)) / out_a;
            let out_b = (b * src_a + dst_pixel[2] as f32 * dst_a * (1.0 - src_a)) / out_a;

            *pixel = Rgba([
                out_r.clamp(0.0, 255.0) as u8,
                out_g.clamp(0.0, 255.0) as u8,
                out_b.clamp(0.0, 255.0) as u8,
                (out_a * 255.0).clamp(0.0, 255.0) as u8,
            ]);
        }
    }

    pub fn get_composite(&mut self) -> &RgbaImage {
        if self.composite_dirty {
            self.composite();
        }
        &self.composite
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Save composite for now
        // Ideally save .ars project file with layers
        self.composite.save(path).context("Failed to save image")?;
        Ok(())
    }

    // API for tools to get raw buffer of active layer
    // Returns None if active layer is not Raster
    pub fn get_active_raster_buffer_mut(&mut self) -> Option<&mut RgbaImage> {
        if let Some(layer) = self.active_layer_mut() {
            match &mut layer.data {
                LayerData::Raster(img) => Some(img),
                LayerData::Tone { buffer, .. } => Some(buffer),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn mark_dirty(&mut self) {
        self.composite_dirty = true;
    }
}
