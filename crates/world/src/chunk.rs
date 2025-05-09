use glam::{IVec2, IVec3, U16Vec3, Vec3, vec3};
use noise::{Fbm, NoiseFn, Perlin};
use owo_colors::OwoColorize;
use std::io::{self, Read};

pub const CHUNK_SIZE: usize = 16;
pub const SUBCHUNK_COUNT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Cube whose size is specified by [`CHUNK_SIZE`] constant.
pub struct SubChunk {
    /// 3D array of block IDs.
    pub blocks: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    /// 3D array of block light level values.
    pub light_levels: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}

impl SubChunk {
    pub const EMPTY: Self = Self {
        blocks: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
        light_levels: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Part of the world consisting of subchunks, number of which is specified by [`SUBCHUNK_COUNT`] constant.
pub struct Chunk {
    /// Chunk location on a 2D grid
    pub origin: IVec2,
    /// Array of chunk vertical sections
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

    pub fn to_local(&self, position: Vec3) -> U16Vec3 {
        let position = position.floor();

        let x = (position.x.rem_euclid(CHUNK_SIZE as f32)) as u16;
        let y = position.y.floor() as u16;
        let z = (position.z.rem_euclid(CHUNK_SIZE as f32)) as u16;

        U16Vec3::new(x, y, z)
    }

    pub const fn get_subchunk_index(&self, y: usize) -> [usize; 2] {
        [y >> 4, y.rem_euclid(CHUNK_SIZE)]
    }

    pub fn to_world(&self, position: U16Vec3) -> IVec3 {
        let IVec2 { x, y } = self.origin;

        IVec3::new(
            (x * CHUNK_SIZE as i32) + i32::from(position.x),
            i32::from(position.y),
            (y * CHUNK_SIZE as i32) + i32::from(position.z),
        )
    }

    pub const fn contains_local_position(&self, position: U16Vec3) -> bool {
        position.x < CHUNK_SIZE as u16
            && position.y < (SUBCHUNK_COUNT * CHUNK_SIZE) as u16
            && position.z < CHUNK_SIZE as u16
    }

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
        let [x, y, z] = self.to_local(position).to_array().map(usize::from);
        let [subchunk, y] = self.get_subchunk_index(y);

        self.subchunks[subchunk].blocks[y][z][x] = block;
    }

    pub fn get_block(&self, position: U16Vec3) -> Option<u8> {
        if !self.contains_local_position(position) {
            return None;
        }

        self.get_block_unchecked(position)
    }

    #[must_use]
    pub fn get_block_unchecked(&self, position: U16Vec3) -> Option<u8> {
        let [x, orig_y, z] = position.to_array().map(usize::from);
        let [subchunk, y] = self.get_subchunk_index(orig_y);

        let block_id = self.subchunks[subchunk].blocks[y][z][x];

        if block_id == 0 { None } else { Some(block_id) }
    }

    pub fn get_subchunk(&self, y: f32) -> Option<&SubChunk> {
        self.subchunks.get((y.floor() as i32 >> 4) as usize)
    }

    pub fn get_subchunk_mut(&mut self, y: f32) -> Option<&mut SubChunk> {
        self.subchunks.get_mut((y.floor() as i32 >> 4) as usize)
    }

    pub fn get_light_level(&self, position: U16Vec3) -> u8 {
        let [x, y, z] = position.to_array().map(usize::from);
        let [subchunk, y] = self.get_subchunk_index(y);

        self.subchunks[subchunk].light_levels[y][z][x]
    }

    pub fn get_light_level_mut(&mut self, position: U16Vec3) -> &mut u8 {
        let [x, y, z] = position.to_array().map(usize::from);
        let [subchunk, y] = self.get_subchunk_index(y);

        &mut self.subchunks[subchunk].light_levels[y][z][x]
    }

    pub fn check_for_block(&self, position: Vec3) -> bool {
        if self.contains_position(position) {
            let [x, y, z] = self.to_local(position).to_array().map(usize::from);
            let [subchunk, y] = self.get_subchunk_index(y);

            self.subchunks[subchunk].blocks[y][z][x] != 0
        } else {
            false
        }
    }

    pub fn get_sun_light(&self, position: U16Vec3) -> u8 {
        (self.get_light_level(position) >> 4) & 0xF
    }

    pub fn set_sun_light(&mut self, position: U16Vec3, value: u8) {
        let level = self.get_light_level_mut(position);

        *level = (*level & 0xF) | (value << 4);
    }

    pub fn get_block_light(&self, position: U16Vec3) -> u8 {
        self.get_light_level(position) & 0xF
    }

    pub fn set_block_light(&mut self, position: U16Vec3, value: u8) {
        let level = self.get_light_level_mut(position);

        *level = (*level & 0xF0) | value;
    }

    pub const fn new(origin: IVec2) -> Self {
        Self {
            origin,
            ..Self::EMPTY
        }
    }

    pub fn generate_surface(&mut self, seed: u32) {
        let generator = Fbm::<Perlin>::new(seed);

        let position = self.origin.as_vec2() * CHUNK_SIZE as f32;
        // let spline = Spline::from_iter([
        //     Key::new(-1.0, 100.0, Interpolation::Cosine),
        //     Key::new(0.3, 100.0, Interpolation::Cosine),
        //     Key::new(0.4, 150.0, Interpolation::Cosine),
        //     Key::new(1.0, 150.0, Interpolation::Cosine),
        // ]);

        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let mut max = 0;

                for y in 0..(CHUNK_SIZE * SUBCHUNK_COUNT) {
                    let value = generator.get([
                        (f64::from(position.x) + x as f64) / CHUNK_SIZE as f64,
                        y as f64 / (CHUNK_SIZE * SUBCHUNK_COUNT) as f64,
                        (f64::from(position.y) + z as f64) / CHUNK_SIZE as f64,
                    ]);

                    if value > 0.0 {
                        max = max.max(y);

                        if y == (CHUNK_SIZE * SUBCHUNK_COUNT - 1) {
                            self.set_block_unchecked(vec3(x as f32, y as f32, z as f32), 2);
                        } else {
                            let value = generator.get([
                                (f64::from(position.x) + x as f64) / CHUNK_SIZE as f64,
                                (y + 1).min((CHUNK_SIZE * SUBCHUNK_COUNT) - 1) as f64
                                    / (CHUNK_SIZE * SUBCHUNK_COUNT) as f64,
                                (f64::from(position.y) + z as f64) / CHUNK_SIZE as f64,
                            ]);

                            if value <= 0.0 {
                                self.set_block_unchecked(vec3(x as f32, y as f32, z as f32), 2);
                            } else {
                                self.set_block_unchecked(vec3(x as f32, y as f32, z as f32), 1);
                            }
                        }
                    }
                }
            }
        }

        println!(
            "[{:18}] Generated chunk at {}",
            "INFO/WorldGen".bright_green(),
            format!("{:>2} {:>2}", self.origin.x, self.origin.y)
                .bright_blue()
                .bold()
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_chunk_serialization() {
        use super::*;

        let mut chunk = Chunk::new(IVec2::new(0, 0));

        chunk.generate_surface(0);

        let serialized = chunk.serialize();

        println!("{}", serialized.len());

        let deserialized = Chunk::deserialize(&serialized).unwrap();

        assert_eq!(chunk.origin, deserialized.origin);
        assert_eq!(chunk.subchunks, deserialized.subchunks);
    }
}
