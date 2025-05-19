use std::{fs, path::Path};

use meralus_world::{BlockModel, Property, TexturePath, TextureRef};

use super::{LoadingError, LoadingResult, ModelLoadingError, texture::TextureLoader};

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

    fn load_block<P: AsRef<Path>, R: AsRef<Path>>(root: R, path: P) -> LoadingResult<BlockModel> {
        let path = path.as_ref().with_extension("json");
        let data = fs::read(&path).map_err(|_| LoadingError::Model(ModelLoadingError::NotFound))?;
        let block = BlockModel::from_slice(&data)
            .map_err(|err| LoadingError::Model(ModelLoadingError::ParsingFailed(err)))?;

        let block = if let Some(parent) = block
            .parent
            .as_ref()
            .and_then(|parent| path.parent().map(|dir| dir.join(parent)))
        {
            let mut parent_block = Self::load_block(root.as_ref(), parent)?;

            parent_block.textures.extend(block.textures);
            parent_block.elements.extend(block.elements);

            parent_block
        } else {
            block
        };

        Ok(block)
    }

    /// # Errors
    ///
    /// An error will be returned if:
    /// - The passed path does not contain a filename.
    /// - The passed path cannot be read.
    /// - The passed path data cannot be successfully parsed.
    /// - An error occurred while loading some texture (see
    ///   [`TextureLoader::load`]).
    pub fn load<P: AsRef<Path>, R: AsRef<Path>>(
        textures: &mut TextureLoader,
        root: R,
        path: P,
    ) -> LoadingResult<BlockModel> {
        let block = Self::load_block(root.as_ref(), path)?;

        for texture_ref in block.textures.values() {
            if let TextureRef::Path(TexturePath(mod_name, path)) = texture_ref
                && mod_name == "game"
            {
                textures.load(
                    root.as_ref()
                        .join("textures")
                        .join(path)
                        .with_extension("png"),
                )?;
            }
        }

        Ok(block)
    }
}
