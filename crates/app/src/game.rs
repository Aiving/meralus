use crate::{Block, Face, VertexExt, get_vertice_neighbours, vertex_ao};
use macroquad::{
    color::{Color, WHITE},
    math::{IVec3, Vec3, Vec4Swizzles, ivec2, ivec3, vec3},
    models::Mesh,
    texture::Texture2D,
    ui::Vertex,
};
use meralus_meshing::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk};
use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    ops::Range,
    path::Path,
};

pub struct Game {
    textures: HashMap<String, Texture2D>,
    blocks: Vec<Block>,
    chunks: HashMap<IVec3, Chunk>,
}

pub struct GameState {
    pub current_block: Option<(u8, IVec3)>,
}

pub struct BackedFace {
    pub position: IVec3,
    pub face: Face,
    pub mesh: Mesh,
}

impl Game {
    #[must_use]
    pub fn new(seed: u32, x_range: Range<i32>, z_range: Range<i32>) -> Self {
        Self {
            textures: HashMap::new(),
            blocks: Vec::new(),
            chunks: x_range
                .flat_map(|x| {
                    z_range.clone().map(move |z| {
                        (
                            IVec3::new(x, 0, z),
                            Chunk::from_perlin_noise(ivec2(x, z), seed),
                        )
                    })
                })
                .collect(),
        }
    }

    pub fn surface_size(&self) -> IVec3 {
        let mut min = IVec3::ZERO;
        let mut max = IVec3::ZERO;

        for chunk in self.chunks.keys() {
            min = min.min(*chunk);
            max = max.max(*chunk);
        }

        (max - min) * 16 + IVec3::new(CHUNK_SIZE as i32, CHUNK_HEIGHT as i32, CHUNK_SIZE as i32)
    }

    pub fn bounds(&self) -> (IVec3, IVec3) {
        let mut min = IVec3::ZERO;
        let mut max = IVec3::ZERO;

        for chunk in self.chunks.keys() {
            min = min.min(*chunk);
            max = max.max(*chunk);
        }

        (min * 16, max * 16)
    }

    pub fn load_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    pub fn load_texture<I: Into<String>, P: AsRef<Path>>(&mut self, id: I, path: P) {
        let id: String = id.into();

        if let Entry::Vacant(entry) = self.textures.entry(id) {
            if let Ok(data) = fs::read(path) {
                let texture = Texture2D::from_file_with_format(&data, None);

                entry.insert(texture);
            }
        }
    }

    pub fn get_texture<I: AsRef<str>>(&self, id: I) -> Option<Texture2D> {
        self.textures.get(id.as_ref()).cloned()
    }

    /// Finds the chunk that contains the given position.
    ///
    /// Algorithm:
    ///
    /// We have:
    ///
    /// Chunks array represented as xz
    /// [(-1; -1) (0; -1) (1; -1)]
    /// [(-1;  0) (0;  0) (1;  0)]
    /// [(-1;  1) (0;  1) (1;  1)]
    ///
    /// where (-1; -1) is 16x16x16 chunk from (-16, 0, -16) to (0, 16, 0)
    /// and where (0; -1) is 16x16x16 chunk from (0, 0, -16) to (16, 16, 0)
    ///
    /// XYZ Position: (-10, 20, 10)
    ///
    #[must_use]
    pub fn find_chunk(&self, position: Vec3) -> Option<&Chunk> {
        self.chunks.get(&ivec3(
            position.x.floor() as i32 >> 4,
            0,
            position.z.floor() as i32 >> 4,
        ))
    }

    pub fn find_chunk_mut(&mut self, position: Vec3) -> Option<&mut Chunk> {
        self.chunks.get_mut(
            &vec3(
                (position.x * (1.0 / CHUNK_SIZE as f32)).floor(),
                0.0,
                (position.z * (1.0 / CHUNK_SIZE as f32)).floor(),
            )
            .as_ivec3(),
        )
    }

    #[must_use]
    pub fn find_block(&self, position: Vec3) -> Option<u8> {
        let chunk = self.find_chunk(position)?;

        chunk.get_block_unchecked(position)
    }

    #[must_use]
    pub fn block_exists(&self, position: Vec3) -> bool {
        self.find_chunk(position)
            .is_some_and(|chunk| chunk.check_for_block(position))
    }

    #[must_use]
    pub fn get_face_mesh(
        &self,
        face: Face,
        position: Vec3,
        texture: Option<Texture2D>,
        color: Option<Color>,
    ) -> Mesh {
        let vertices = face.as_vertices();
        let uv = face.as_uv();
        let normal = face.as_normal();

        let vertices: Vec<Vertex> = (0..4)
            .map(|i| {
                let (vertice_neighbours, extra_vertice_neighbours) = get_vertice_neighbours(
                    position,
                    vertices[i].y > 0.0,
                    vertices[i].x > 0.0,
                    vertices[i].z > 0.0,
                );

                let [side1, side2, corner] =
                    vertice_neighbours.map(|pos| self.find_block(pos).is_some());

                let ambient_occlusion = vertex_ao(
                    side1,
                    side2,
                    corner,
                    extra_vertice_neighbours.is_some_and(|vertice_neighbours| {
                        let [side1, side2, side3] =
                            vertice_neighbours.map(|pos| self.find_block(pos).is_some());

                        (side1 || side2) && side3
                    }),
                );

                let color = (WHITE.to_vec().xyz() * ambient_occlusion).extend(1.0);

                Vertex::new2(position + vertices[i], uv[i], Color::from_vec(color))
                    .with_normal(normal.extend(ambient_occlusion))
            })
            .collect();

        let indices = if vertices[1].normal.w + vertices[3].normal.w
            > vertices[0].normal.w + vertices[2].normal.w
        {
            // FLIP!
            vec![3, 2, 1, 1, 0, 3]
        } else {
            vec![0, 1, 2, 2, 3, 0]
        };

        let mut mesh = Mesh {
            vertices,
            indices,
            texture,
        };

        if let Some(color) = color {
            for vertex in &mut mesh.vertices {
                let color0 = Color::from(vertex.color);

                vertex.color =
                    Color::from_vec((color0.to_vec().xyz() * color.to_vec().xyz()).extend(1.0))
                        .into();
            }
        }

        mesh
    }

    #[must_use]
    pub fn bake_face(
        &self,
        position: IVec3,
        face: Face,
        texture: Texture2D,
        color: Option<Color>,
    ) -> BackedFace {
        BackedFace {
            position,
            face,
            mesh: self.get_face_mesh(face, position.as_vec3(), Some(texture), color),
        }
    }

    #[must_use]
    pub fn compute_world_mesh(&self) -> Vec<BackedFace> {
        let mut meshes = Vec::new();

        for chunk in self.chunks.values() {
            for y in 0..CHUNK_HEIGHT {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let block_id = chunk.blocks[y][z][x];

                        if block_id != 0 {
                            let block = self.blocks.get(usize::from(block_id - 1)).unwrap();

                            let pos = vec3(chunk.origin.x as f32, 0.0, chunk.origin.y as f32)
                                * 16.0
                                + Vec3::new(x as f32, y as f32, z as f32);

                            let position = ivec3(chunk.origin.x, 0, chunk.origin.y) * 16
                                + ivec3(x as i32, y as i32, z as i32);

                            for face in Face::ALL {
                                if !self.block_exists(pos + face.as_normal()) {
                                    for (texture, color) in block.get_face_textures(face) {
                                        meshes.push(self.bake_face(position, face, texture, color));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        meshes
    }
}
