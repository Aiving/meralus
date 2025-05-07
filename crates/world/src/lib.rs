#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::unreadable_literal,
    clippy::missing_errors_doc
)]

mod block;
mod chunk;
mod chunk_manager;

pub use self::{
    block::{
        Axis, BlockCondition, BlockElement, BlockFace, BlockModel, BlockState, BlockStates,
        ConditionValue, Corner, Face, Faces, Property, PropertyValue, TextureId,
    },
    chunk::{CHUNK_SIZE, Chunk, SUBCHUNK_COUNT, SubChunk},
    chunk_manager::ChunkManager,
};
