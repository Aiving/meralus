use glam::{IVec2, U16Vec3, Vec3, vec3};
use noise::{NoiseFn, Perlin};
use owo_colors::OwoColorize;
use std::io::{self, Read};

pub const CHUNK_SIZE: usize = 16;
pub const SUBCHUNK_COUNT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubChunk {
    pub blocks: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    pub light_levels: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}

impl SubChunk {
    pub const EMPTY: Self = Self {
        blocks: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
        light_levels: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub origin: IVec2,
    pub subchunks: [SubChunk; SUBCHUNK_COUNT],
}

impl Chunk {
    pub fn deserialize<T: AsRef<[u8]>>(data: T) -> io::Result<Self> {
        let mut value = Self::EMPTY;

        let mut data = data.as_ref();

        value.origin = {
            let mut x = [0; 4];
            let mut z = [0; 4];

            data.read_exact(&mut x)?;
            data.read_exact(&mut z)?;

            let x = i32::from_be_bytes(x);
            let z = i32::from_be_bytes(z);

            IVec2::new(x, z)
        };

        // for y in 0..CHUNK_HEIGHT {
        //     for z in 0..CHUNK_SIZE {
        //         for x in 0..CHUNK_SIZE {
        //             let mut buf = [0; 2];

        //             data.read_exact(&mut buf)?;

        //             value.blocks[y][z][x] = buf[0];
        //             value.light_levels[y][z][x] = buf[1];
        //         }
        //     }
        // }

        Ok(value)
    }

    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();

        data.extend_from_slice(&self.origin.x.to_be_bytes());
        data.extend_from_slice(&self.origin.y.to_be_bytes());

        // for y in 0..CHUNK_HEIGHT {
        //     for z in 0..CHUNK_SIZE {
        //         for x in 0..CHUNK_SIZE {
        //             data.push(self.blocks[y][z][x]);
        //             data.push(self.light_levels[y][z][x]);
        //         }
        //     }
        // }

        data
    }

    #[allow(clippy::large_stack_arrays)]
    pub const EMPTY: Self = Self {
        origin: IVec2::ZERO,
        subchunks: [SubChunk::EMPTY; SUBCHUNK_COUNT],
    };

    pub fn contains_position(&self, position: Vec3) -> bool {
        self.origin.x == (position.x.floor() as i32 >> 4)
            && self.origin.y == (position.z.floor() as i32 >> 4)
            && (0..SUBCHUNK_COUNT).contains(&((position.y.floor() as i32 >> 4) as usize))
    }

    pub fn set_block(&mut self, position: Vec3, block: u8) {
        if self.contains_position(position) {
            self.set_block_unchecked(position, block);
        }
    }

    pub fn set_block_unchecked(&mut self, position: Vec3, block: u8) {
        let position = position.floor();

        let x = (position.x.rem_euclid(CHUNK_SIZE as f32)) as usize;
        let subchunk = (position.y as i32 >> 4) as usize;
        let y = (position.y.rem_euclid(CHUNK_SIZE as f32)) as usize;
        let z = (position.z.rem_euclid(CHUNK_SIZE as f32)) as usize;

        self.subchunks[subchunk].blocks[y][z][x] = block;
    }

    pub fn get_block_inner(&self, position: U16Vec3) -> Option<u8> {
        let [x, y, z] = position.to_array().map(usize::from);

        let subchunk = y >> 4;
        let subchunk_y = y.rem_euclid(CHUNK_SIZE);

        if x >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return None;
        }

        self.subchunks
            .get(subchunk)
            .and_then(|subchunk| {
                subchunk
                    .blocks
                    .get(subchunk_y)
                    .and_then(|z_row| z_row.get(z).and_then(|x_row| x_row.get(x)))
            })
            .filter(|value| value != &&u8::MIN)
            .copied()
    }

    pub fn get_block(&self, position: Vec3) -> Option<u8> {
        if !self.contains_position(position) {
            return None;
        }

        self.get_block_unchecked(position)
    }

    #[must_use]
    pub fn get_block_unchecked(&self, position: Vec3) -> Option<u8> {
        let position = position.floor();

        let x = (position.x.rem_euclid(CHUNK_SIZE as f32)) as usize;
        let subchunk = (position.y as i32 >> 4) as usize;
        let y = (position.y.rem_euclid(CHUNK_SIZE as f32)) as usize;
        let z = (position.z.rem_euclid(CHUNK_SIZE as f32)) as usize;

        let block_id = self.subchunks[subchunk].blocks[y][z][x];

        if block_id == 0 { None } else { Some(block_id) }
    }

    pub fn get_subchunk(&self, y: f32) -> Option<&SubChunk> {
        self.subchunks.get((y.floor() as i32 >> 4) as usize)
    }

    pub fn get_subchunk_mut(&mut self, y: f32) -> Option<&mut SubChunk> {
        self.subchunks.get_mut((y.floor() as i32 >> 4) as usize)
    }

    #[must_use]
    pub fn get_light_level(&self, position: Vec3) -> Option<u8> {
        if !self.contains_position(position) {
            return None;
        }

        let x = (position.x.rem_euclid(CHUNK_SIZE as f32)) as usize;
        let subchunk = (position.y as i32 >> 4) as usize;
        let y = (position.y.rem_euclid(CHUNK_SIZE as f32)) as usize;
        let z = (position.z.rem_euclid(CHUNK_SIZE as f32)) as usize;

        if self.subchunks[subchunk].blocks[y][z][x] == 0 {
            None
        } else {
            Some(self.subchunks[subchunk].light_levels[y][z][x])
        }
    }

    #[must_use]
    pub fn check_for_block(&self, position: Vec3) -> bool {
        if self.contains_position(position) {
            let x = (position.x.rem_euclid(CHUNK_SIZE as f32)) as usize;
            let subchunk = (position.y as i32 >> 4) as usize;
            let y = (position.y.rem_euclid(CHUNK_SIZE as f32)) as usize;
            let z = (position.z.rem_euclid(CHUNK_SIZE as f32)) as usize;

            self.subchunks[subchunk].blocks[y][z][x] != 0
        } else {
            false
        }
    }

    #[must_use]
    pub fn get_sun_light(&self, position: Vec3) -> Option<u8> {
        self.get_light_level(position)
            .map(|level| (level >> 4) & 0xF)
    }

    #[must_use]
    pub fn get_block_light(&self, position: Vec3) -> Option<u8> {
        self.get_light_level(position).map(|level| level & 0xF)
    }

    #[must_use]
    pub fn from_perlin_noise(origin: IVec2, seed: u32) -> Self {
        let generator = Perlin::new(seed);
        let position = origin.as_vec2() * CHUNK_SIZE as f32;
        // let spline = Spline::from_iter([
        //     Key::new(-1.0, 100.0, Interpolation::Cosine),
        //     Key::new(0.3, 100.0, Interpolation::Cosine),
        //     Key::new(0.4, 150.0, Interpolation::Cosine),
        //     Key::new(1.0, 150.0, Interpolation::Cosine),
        // ]);

        let mut empty = Self::EMPTY;

        empty.origin = origin;

        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                for y in 0..(CHUNK_SIZE * SUBCHUNK_COUNT) {
                    let value = generator.get([
                        (f64::from(position.x) + x as f64) / CHUNK_SIZE as f64,
                        y as f64 / (CHUNK_SIZE * SUBCHUNK_COUNT) as f64,
                        (f64::from(position.y) + z as f64) / CHUNK_SIZE as f64,
                    ]);

                    if value > 0.0 {
                        empty.set_block_unchecked(vec3(x as f32, y as f32, z as f32), 1);
                    }
                }
            }
        }

        println!(
            "[{:18}] Generated chunk at {}",
            "INFO/WorldGen".bright_green(),
            format!("{:>2} {:>2}", origin.x, origin.y)
                .bright_blue()
                .bold()
        );

        empty
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_chunk_serialization() {
        use super::*;

        let chunk = Chunk::from_perlin_noise(IVec2::new(0, 0), 0);
        let serialized = chunk.serialize();
        println!("{}", serialized.len());
        let deserialized = Chunk::deserialize(&serialized).unwrap();

        assert_eq!(chunk.origin, deserialized.origin);
        assert_eq!(chunk.subchunks, deserialized.subchunks);
    }
}
