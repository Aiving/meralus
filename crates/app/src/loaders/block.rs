use super::texture::TextureLoader;
use meralus_world::{Block, TextureId};
use std::fs;
use std::path::Path;

pub struct BlockLoader;

impl BlockLoader {
    pub fn load<P: AsRef<Path>, R: AsRef<Path>>(
        textures: &mut TextureLoader,
        root: R,
        path: P,
    ) -> Option<Block> {
        let path = path.as_ref().with_extension("json");
        let data = fs::read(&path).ok()?;
        let block = Block::from_slice(&data).ok()?;

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
