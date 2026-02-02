use crate::image_store::ImageStore;
use crate::layers::LayerData;
use image::{GenericImage, RgbaImage};

pub trait Command {
    fn undo(&self, image: &mut ImageStore);
    fn redo(&self, image: &mut ImageStore);
    fn name(&self) -> &str;
}

pub struct CommandStack {
    commands: Vec<Box<dyn Command>>,
    cursor: usize,
}

impl CommandStack {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            cursor: 0,
        }
    }

    pub fn push(&mut self, command: Box<dyn Command>) {
        if self.cursor < self.commands.len() {
            self.commands.truncate(self.cursor);
        }
        self.commands.push(command);
        self.cursor += 1;
    }

    pub fn undo(&mut self, image: &mut ImageStore) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.commands[self.cursor].undo(image);
            image.mark_dirty();
        }
    }

    pub fn redo(&mut self, image: &mut ImageStore) {
        if self.cursor < self.commands.len() {
            self.commands[self.cursor].redo(image);
            self.cursor += 1;
            image.mark_dirty();
        }
    }

    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        self.cursor < self.commands.len()
    }
}

pub struct PatchCommand {
    #[allow(dead_code)]
    pub name: String,
    pub layer_index: usize,
    pub x: u32,
    pub y: u32,
    pub old_patch: RgbaImage,
    pub new_patch: RgbaImage,
}

impl Command for PatchCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn undo(&self, image: &mut ImageStore) {
        if let Some(layer) = image.layers.get_mut(self.layer_index) {
            match &mut layer.data {
                LayerData::Raster(img) | LayerData::Tone { buffer: img, .. } => {
                    let _ = img.copy_from(&self.old_patch, self.x, self.y);
                }
                _ => {} // Vector undo not implemented in PatchCommand
            }
        }
    }

    fn redo(&self, image: &mut ImageStore) {
        if let Some(layer) = image.layers.get_mut(self.layer_index) {
            match &mut layer.data {
                LayerData::Raster(img) | LayerData::Tone { buffer: img, .. } => {
                    let _ = img.copy_from(&self.new_patch, self.x, self.y);
                }
                _ => {}
            }
        }
    }
}
