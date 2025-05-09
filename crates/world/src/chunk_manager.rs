use crate::{CHUNK_SIZE, Chunk, SUBCHUNK_COUNT};
use glam::{IVec2, IVec3, Vec3};
use std::collections::HashMap;

pub struct ChunkManager {
    chunks: HashMap<IVec2, Chunk>,
}

impl ChunkManager {
    pub fn from_range<T: Iterator<Item = i32> + Clone>(x: T, z: &T) -> Self {
        Self {
            chunks: x
                .flat_map(|x| {
                    z.clone().map(move |z| {
                        let origin = IVec2::new(x, z);

                        (origin, Chunk::new(origin))
                    })
                })
                .collect(),
        }
    }

    pub fn generate_surface(&mut self, seed: u32) {
        for chunk in self.chunks_mut() {
            chunk.generate_surface(seed);
        }
    }

    pub fn surface_size(&self) -> IVec3 {
        let mut min = IVec2::ZERO;
        let mut max = IVec2::ZERO;

        for chunk in self.chunks.keys() {
            min = min.min(*chunk);
            max = max.max(*chunk);
        }

        let size = (max - min) * 16;

        IVec3::new(
            size.x + CHUNK_SIZE as i32,
            (CHUNK_SIZE * SUBCHUNK_COUNT) as i32,
            size.y + CHUNK_SIZE as i32,
        )
    }

    pub fn bounds(&self) -> (IVec2, IVec2) {
        let mut min = IVec2::ZERO;
        let mut max = IVec2::ZERO;

        for chunk in self.chunks.keys() {
            min = min.min(*chunk);
            max = max.max(*chunk);
        }

        (min * CHUNK_SIZE as i32, max * CHUNK_SIZE as i32)
    }

    pub fn to_local(position: Vec3) -> IVec2 {
        IVec2::new(
            position.x.floor() as i32 >> 4,
            position.z.floor() as i32 >> 4,
        )
    }

    pub fn get_chunk(&self, position: &IVec2) -> Option<&Chunk> {
        self.chunks.get(position)
    }

    pub fn get_chunk_mut(&mut self, position: &IVec2) -> Option<&mut Chunk> {
        self.chunks.get_mut(position)
    }

    pub fn get_block(&self, position: Vec3) -> Option<u8> {
        let chunk = self.get_chunk(&Self::to_local(position))?;

        chunk.get_block(chunk.to_local(position))
    }

    pub fn contains_block(&self, position: Vec3) -> bool {
        self.get_chunk(&Self::to_local(position))
            .is_some_and(|chunk| chunk.check_for_block(position))
    }

    pub fn contains_chunk(&self, position: Vec3) -> bool {
        self.chunks.contains_key(&IVec2::new(
            position.x.floor() as i32 >> 4,
            position.z.floor() as i32 >> 4,
        ))
    }

    pub fn get_block_light(&self, position: Vec3) -> u8 {
        self.get_chunk(&Self::to_local(position))
            .map_or(15, |chunk| {
                let local_position = chunk.to_local(position);

                if chunk.contains_local_position(local_position) {
                    chunk.get_block_light(local_position)
                } else {
                    15
                }
            })
    }

    pub fn get_sun_light(&self, position: Vec3) -> u8 {
        self.get_chunk(&Self::to_local(position))
            .map_or(15, |chunk| {
                let local_position = chunk.to_local(position);

                if chunk.contains_local_position(local_position) {
                    chunk.get_sun_light(local_position)
                } else {
                    15
                }
            })
    }

    pub fn get_light(&self, position: Vec3) -> u8 {
        self.get_chunk(&Self::to_local(position))
            .map_or(240, |chunk| {
                let local_position = chunk.to_local(position);

                if chunk.contains_local_position(local_position) {
                    chunk.get_light_level(local_position)
                } else {
                    240
                }
            })
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn chunks(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.values()
    }

    pub fn chunks_mut(&mut self) -> impl Iterator<Item = &mut Chunk> {
        self.chunks.values_mut()
    }
}
