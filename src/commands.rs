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

    pub fn entries(&self) -> &[Box<dyn Command>] {
        &self.commands
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Undo/redo until `cursor == index` (clamped to `0..=len`).
    pub fn jump_to(&mut self, index: usize, image: &mut ImageStore) {
        let target = index.min(self.commands.len());
        while self.cursor > target {
            self.cursor -= 1;
            self.commands[self.cursor].undo(image);
        }
        while self.cursor < target {
            self.commands[self.cursor].redo(image);
            self.cursor += 1;
        }
        image.mark_dirty();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_store::ImageStore;

    // Minimal no-op command that records its name; undo/redo do nothing to the image.
    struct NamedCmd(&'static str);
    impl Command for NamedCmd {
        fn undo(&self, _image: &mut ImageStore) {}
        fn redo(&self, _image: &mut ImageStore) {}
        fn name(&self) -> &str { self.0 }
    }

    fn stack_with(names: &[&'static str]) -> CommandStack {
        let mut s = CommandStack::new();
        for n in names { s.push(Box::new(NamedCmd(n))); }
        s
    }

    #[test]
    fn entries_and_cursor_track_pushes() {
        let s = stack_with(&["a", "b", "c"]);
        assert_eq!(s.entries().len(), 3);
        assert_eq!(s.cursor(), 3);
        assert_eq!(s.entries()[0].name(), "a");
    }

    #[test]
    fn jump_to_earlier_index_moves_cursor_back() {
        let mut img = ImageStore::new(4, 4);
        let mut s = stack_with(&["a", "b", "c"]);
        s.jump_to(1, &mut img); // keep only "a" applied
        assert_eq!(s.cursor(), 1);
    }

    #[test]
    fn jump_to_later_index_moves_cursor_forward() {
        let mut img = ImageStore::new(4, 4);
        let mut s = stack_with(&["a", "b", "c"]);
        s.jump_to(0, &mut img);
        s.jump_to(3, &mut img);
        assert_eq!(s.cursor(), 3);
    }

    #[test]
    fn jump_to_clamps_out_of_range() {
        let mut img = ImageStore::new(4, 4);
        let mut s = stack_with(&["a", "b"]);
        s.jump_to(99, &mut img);
        assert_eq!(s.cursor(), 2);
    }
}
