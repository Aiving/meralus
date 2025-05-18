mod block;
mod block_model;
mod block_states;
mod texture;

pub use self::{
    block::{Block, BlockManager},
    block_model::{BakedBlockModel, BakedBlockModelLoader, ModelLoadingError},
    texture::{TextureAtlas, TextureLoader, TextureLoadingError},
};

pub type LoadingResult<T> = Result<T, LoadingError>;

#[derive(Debug)]
pub enum LoadingError {
    Texture(TextureLoadingError),
    Model(ModelLoadingError),
}
