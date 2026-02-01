use crate::image_store::ImageStore;
use image::{GenericImage, RgbaImage};

pub trait Command {
    fn undo(&self, image: &mut ImageStore);
    fn redo(&self, image: &mut ImageStore);
    fn name(&self) -> &str;
}

pub struct CommandStack {
    commands: Vec<Box<dyn Command>>,
    cursor: usize, // Points to the slot for the *next* command (or after the last executed one)
}

impl CommandStack {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            cursor: 0,
        }
    }

    pub fn push(&mut self, command: Box<dyn Command>) {
        // If we are in the middle of the stack, drop all future commands (no redo after new action)
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
        }
    }

    pub fn redo(&mut self, image: &mut ImageStore) {
        if self.cursor < self.commands.len() {
            self.commands[self.cursor].redo(image);
            self.cursor += 1;
        }
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor < self.commands.len()
    }
}

// A generic command that stores a rectangular patch of the image before and after the operation
pub struct PatchCommand {
    pub name: String,
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
        let _ = image.buffer.copy_from(&self.old_patch, self.x, self.y);
    }

    fn redo(&self, image: &mut ImageStore) {
        let _ = image.buffer.copy_from(&self.new_patch, self.x, self.y);
    }
}
