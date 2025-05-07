use super::texture::TextureLoader;
use meralus_world::{BlockModel, Property, TextureId};
use std::fs;
use std::path::Path;

pub trait Block {
    fn get_properties(&self) -> Vec<Property>;
}

pub struct BlockManager {
    blocks: Vec<Box<dyn Block>>,
}

impl BlockManager {
    pub fn register<T: Block + 'static>(&mut self, block: T) {
        self.blocks.push(Box::new(block) as Box<dyn Block>);
    }

    pub fn load<P: AsRef<Path>, R: AsRef<Path>>(
        textures: &mut TextureLoader,
        root: R,
        path: P,
    ) -> Option<BlockModel> {
        let path = path.as_ref().with_extension("json");
        let data = fs::read(&path).ok()?;
        let block = BlockModel::from_slice(&data).ok()?;

        for TextureId(mod_name, path) in block.textures.values() {
            if mod_name == "game" {
                textures.load(
                    root.as_ref()
                        .join("textures")
                        .join(path)
                        .with_extension("png"),
                );
            }
        }

        Some(block)
    }
}
