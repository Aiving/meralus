use glam::{IVec2, IVec3, Vec3};
use noise::{NoiseFn, Perlin};
use std::{
    array,
    io::{self, Read, Write},
};

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = CHUNK_SIZE * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub origin: IVec2,
    pub blocks: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_HEIGHT],
    pub light_levels: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_HEIGHT],
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

        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let mut buf = [0; 2];

                    data.read_exact(&mut buf)?;

                    value.blocks[y][z][x] = buf[0];
                    value.light_levels[y][z][x] = buf[1];
                }
            }
        }

        Ok(value)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();

        data.extend_from_slice(&self.origin.x.to_be_bytes());
        data.extend_from_slice(&self.origin.y.to_be_bytes());

        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    data.push(self.blocks[y][z][x]);
                    data.push(self.light_levels[y][z][x]);
                }
            }
        }
        data
    }

    pub const EMPTY: Self = Self {
        origin: IVec2::ZERO,
        blocks: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_HEIGHT],
        light_levels: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_HEIGHT],
    };

    fn generate_block(perlin_value: f64, y: usize, block_up: u8) -> u8 {
        if y > 15 {
            0
        } else if perlin_value > 0.0 {
            if block_up != 0 { 1 } else { 2 }
        } else {
            0
        }
    }

    pub fn contains_position(&self, position: Vec3) -> bool {
        self.origin.x == (position.x.floor() as i32 >> 4)
            && self.origin.y == (position.z.floor() as i32 >> 4)
    }

    pub fn get_block(&self, position: Vec3) -> Option<u8> {
        if !self.contains_position(position) {
            return None;
        }

        let x = (position.x.floor().rem_euclid(CHUNK_SIZE as f32)) as usize;
        let y = (position.y.floor() % CHUNK_HEIGHT as f32) as usize;
        let z = (position.z.floor().rem_euclid(CHUNK_SIZE as f32)) as usize;

        let block_id = self.blocks[y][z][x];

        if block_id == 0 { None } else { Some(block_id) }
    }

    pub fn get_block_unchecked(&self, position: Vec3) -> Option<u8> {
        let x = (position.x.floor().rem_euclid(CHUNK_SIZE as f32)) as usize;
        let y = (position.y.floor() % CHUNK_HEIGHT as f32) as usize;
        let z = (position.z.floor().rem_euclid(CHUNK_SIZE as f32)) as usize;

        let block_id = self.blocks[y][z][x];

        if block_id == 0 { None } else { Some(block_id) }
    }

    pub fn get_light_level(&self, position: Vec3) -> Option<u8> {
        if !self.contains_position(position) {
            return None;
        }

        let x = (position.x.floor().rem_euclid(CHUNK_SIZE as f32)) as usize;
        let y = (position.y.floor() % CHUNK_HEIGHT as f32) as usize;
        let z = (position.z.floor().rem_euclid(CHUNK_SIZE as f32)) as usize;

        if self.blocks[y][z][x] == 0 {
            None
        } else {
            Some(self.light_levels[y][z][x])
        }
    }

    pub fn get_sun_light(&self, position: Vec3) -> Option<u8> {
        self.get_light_level(position)
            .map(|level| (level >> 4) & 0xF)
    }

    pub fn get_block_light(&self, position: Vec3) -> Option<u8> {
        self.get_light_level(position).map(|level| level & 0xF)
    }

    #[must_use]
    pub fn from_perlin_noise(origin: IVec2, seed: u32) -> Self {
        let generator = Perlin::new(seed);
        let position = origin.as_vec2();

        Self {
            origin,
            blocks: array::from_fn(|y| {
                array::from_fn(|z| {
                    array::from_fn(|x| {
                        let value = generator.get([
                            (f64::from(position.x) + x as f64) / CHUNK_SIZE as f64,
                            y as f64 / CHUNK_HEIGHT as f64,
                            (f64::from(position.y) + z as f64) / CHUNK_SIZE as f64,
                        ]);

                        let up_value = generator.get([
                            (f64::from(position.x) + x as f64) / CHUNK_SIZE as f64,
                            (y as f64 + 1.0) / CHUNK_HEIGHT as f64,
                            (f64::from(position.y) + z as f64) / CHUNK_SIZE as f64,
                        ]);

                        let block_up = Self::generate_block(up_value, y + 1, 0);

                        Self::generate_block(value, y, block_up)
                    })
                })
            }),
            light_levels: array::from_fn(|y| array::from_fn(|z| array::from_fn(|x| 0))),
        }
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
        assert_eq!(chunk.blocks, deserialized.blocks);
        assert_eq!(chunk.light_levels, deserialized.light_levels);
    }
}
