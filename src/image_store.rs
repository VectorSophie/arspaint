use crate::layers::{BlendMode, Layer};
use anyhow::{Context, Result};
use image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use std::path::Path;

#[derive(Clone)]
pub struct ImageStore {
    width: u32,
    height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    pub selection: Option<image::GrayImage>,
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
            selection: None,
            composite: ImageBuffer::new(width, height),
            composite_dirty: true,
        };

        // Fill first layer with white
        for pixel in store.layers[0].pixels.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
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
            pixels: buffer,
        };

        let mut store = Self {
            width,
            height,
            layers: vec![layer],
            active_layer: 0,
            selection: None,
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
                Some(&layers[i - 1].pixels)
            } else {
                None
            };
            Self::blend_buffer_static(dest, &layer.pixels, layer.opacity, layer.blend, mask);
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

            let sf = |c: u8| c as f32 / 255.0;
            let (sr, sg, sb) = (sf(src_pixel[0]), sf(src_pixel[1]), sf(src_pixel[2]));
            let (dr, dg, db) = (sf(dst_pixel[0]), sf(dst_pixel[1]), sf(dst_pixel[2]));

            let overlay_ch = |s: f32, d: f32| -> f32 {
                if d < 0.5 { 2.0 * s * d } else { 1.0 - 2.0 * (1.0 - s) * (1.0 - d) }
            };
            let soft_light_ch = |s: f32, d: f32| -> f32 {
                if s < 0.5 { d - (1.0 - 2.0*s) * d * (1.0 - d) }
                else { d + (2.0*s - 1.0) * (if d < 0.25 { ((16.0*d - 12.0)*d + 4.0)*d } else { d.sqrt() } - d) }
            };

            let (r, g, b) = match mode {
                BlendMode::Normal   => (sr * 255.0, sg * 255.0, sb * 255.0),
                BlendMode::Multiply => (sr * dr * 255.0, sg * dg * 255.0, sb * db * 255.0),
                BlendMode::Add      => ((sr + dr).min(1.0) * 255.0, (sg + dg).min(1.0) * 255.0, (sb + db).min(1.0) * 255.0),
                BlendMode::Screen   => ((1.0-(1.0-sr)*(1.0-dr))*255.0, (1.0-(1.0-sg)*(1.0-dg))*255.0, (1.0-(1.0-sb)*(1.0-db))*255.0),
                BlendMode::Overlay  => (overlay_ch(sr,dr)*255.0, overlay_ch(sg,dg)*255.0, overlay_ch(sb,db)*255.0),
                BlendMode::SoftLight=> (soft_light_ch(sr,dr)*255.0, soft_light_ch(sg,dg)*255.0, soft_light_ch(sb,db)*255.0),
                BlendMode::Difference => ((sr-dr).abs()*255.0, (sg-dg).abs()*255.0, (sb-db).abs()*255.0),
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
        self.active_layer_mut().map(|layer| &mut layer.pixels)
    }

    pub fn mark_dirty(&mut self) {
        self.composite_dirty = true;
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == self.width && new_height == self.height {
            return;
        }

        for (idx, layer) in self.layers.iter_mut().enumerate() {
            let mut new_img = ImageBuffer::new(new_width, new_height);
            if idx == 0 {
                for p in new_img.pixels_mut() {
                    *p = Rgba([255, 255, 255, 255]);
                }
            }
            let copy_w = self.width.min(new_width);
            let copy_h = self.height.min(new_height);
            for y in 0..copy_h {
                for x in 0..copy_w {
                    new_img.put_pixel(x, y, *layer.pixels.get_pixel(x, y));
                }
            }
            layer.pixels = new_img;
        }

        if let Some(mask) = &mut self.selection {
            let mut new_mask = ImageBuffer::new(new_width, new_height);
            let copy_w = self.width.min(new_width);
            let copy_h = self.height.min(new_height);
            for y in 0..copy_h {
                for x in 0..copy_w {
                    new_mask.put_pixel(x, y, *mask.get_pixel(x, y));
                }
            }
            *mask = new_mask;
        }

        self.width = new_width;
        self.height = new_height;
        self.composite = ImageBuffer::new(new_width, new_height);
        self.mark_dirty();
    }

    pub fn resize_scaled(&mut self, new_width: u32, new_height: u32) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            let src = image::imageops::resize(buf, new_width, new_height, image::imageops::FilterType::Nearest);
            *buf = src;
        }
        self.width = new_width;
        self.height = new_height;
        self.composite = ImageBuffer::new(new_width, new_height);
        self.mark_dirty();
    }

    pub fn rotate90_cw(&mut self) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            *buf = image::imageops::rotate90(buf);
        }
        std::mem::swap(&mut self.width, &mut self.height);
        self.composite = ImageBuffer::new(self.width, self.height);
        self.mark_dirty();
    }

    pub fn rotate90_ccw(&mut self) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            *buf = image::imageops::rotate270(buf);
        }
        std::mem::swap(&mut self.width, &mut self.height);
        self.composite = ImageBuffer::new(self.width, self.height);
        self.mark_dirty();
    }

    pub fn rotate180(&mut self) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            *buf = image::imageops::rotate180(buf);
        }
        self.mark_dirty();
    }

    pub fn flip_horizontal(&mut self) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            image::imageops::flip_horizontal_in_place(buf);
        }
        self.mark_dirty();
    }

    pub fn flip_vertical(&mut self) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            image::imageops::flip_vertical_in_place(buf);
        }
        self.mark_dirty();
    }

    pub fn invert_colors(&mut self) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            for pixel in buf.pixels_mut() {
                pixel[0] = 255 - pixel[0];
                pixel[1] = 255 - pixel[1];
                pixel[2] = 255 - pixel[2];
            }
        }
        self.mark_dirty();
    }

    pub fn crop_to(&mut self, x: u32, y: u32, w: u32, h: u32) {
        if let Some(buf) = self.get_active_raster_buffer_mut() {
            use image::GenericImageView;
            *buf = buf.view(x, y, w, h).to_image();
        }
        self.width = w;
        self.height = h;
        self.selection = None;
        self.composite = ImageBuffer::new(w, h);
        self.mark_dirty();
    }

    pub fn merge_down(&mut self) {
        let idx = self.active_layer;
        if idx == 0 || self.layers.len() < 2 { return; }

        // composite active layer onto the one below
        let above = self.layers[idx].clone();
        let below = &mut self.layers[idx - 1];

        let src = &above.pixels;
        let dst = &mut below.pixels;
        for (x, y, sp) in src.enumerate_pixels() {
            if sp[3] == 0 { continue; }
            let src_a = (sp[3] as f32 / 255.0) * above.opacity;
            let dp = dst.get_pixel(x, y);
            let dst_a = dp[3] as f32 / 255.0;
            let out_a = src_a + dst_a * (1.0 - src_a);
            if out_a == 0.0 { continue; }
            let blend = |s: u8, d: u8| -> u8 {
                ((s as f32 * src_a + d as f32 * dst_a * (1.0 - src_a)) / out_a).clamp(0.0, 255.0) as u8
            };
            dst.put_pixel(x, y, image::Rgba([blend(sp[0],dp[0]),blend(sp[1],dp[1]),blend(sp[2],dp[2]),(out_a*255.0) as u8]));
        }

        self.layers.remove(idx);
        self.active_layer = idx - 1;
        self.mark_dirty();
    }

    pub fn get_active_raster_snapshot(&self) -> Option<RgbaImage> {
        self.layers.get(self.active_layer).map(|l| l.pixels.clone())
    }
}
