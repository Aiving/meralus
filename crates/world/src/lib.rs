#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::unreadable_literal,
    clippy::missing_errors_doc
)]

mod block;
mod chunk;

pub use self::{
    block::{Axis, Block, BlockElement, BlockFace, Face, Faces, TextureId},
    chunk::{CHUNK_SIZE, Chunk, SUBCHUNK_COUNT, SubChunk},
};
