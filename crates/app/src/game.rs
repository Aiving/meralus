use crate::{BlockModelLoader, TextureLoader, mesh::Mesh, vertex_ao};
use glam::{IVec3, Vec3, ivec2, ivec3, u16vec3, vec3};
use meralus_engine::{AsValue, Color, Vertex, WindowDisplay, glium::texture::CompressedTexture2d};
use meralus_world::{CHUNK_SIZE, Chunk, Face, SUBCHUNK_COUNT};
use owo_colors::OwoColorize;
use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
};

pub struct Game {
    textures: TextureLoader,
    blocks: BlockModelLoader,
    chunks: HashMap<IVec3, Chunk>,
    players: Vec<Player>,
    root: PathBuf,
    // pub egui: EGui,
}

pub struct Player {
    pub position: Vec3,
    pub nickname: String,
    pub is_me: bool,
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
    pub fn new(
        root: impl Into<PathBuf>,
        seed: u32,
        x_range: Range<i32>,
        z_range: Range<i32>,
    ) -> Self {
        Self {
            textures: TextureLoader::default(),
            blocks: BlockModelLoader::default(),
            players: Vec::new(),
            root: root.into(),
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

    pub fn chunks_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn surface_size(&self) -> IVec3 {
        let mut min = IVec3::ZERO;
        let mut max = IVec3::ZERO;

        for chunk in self.chunks.keys() {
            min = min.min(*chunk);
            max = max.max(*chunk);
        }

        (max - min) * 16
            + IVec3::new(
                CHUNK_SIZE as i32,
                (CHUNK_SIZE * SUBCHUNK_COUNT) as i32,
                CHUNK_SIZE as i32,
            )
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

    pub fn load_block<P: AsRef<Path>>(&mut self, display: &WindowDisplay, path: P) {
        self.blocks
            .load(&mut self.textures, display, &self.root, path);
    }

    pub fn load_buitlin_blocks(&mut self, display: &WindowDisplay) {
        if let Ok(mut root) = self.root.join("models").read_dir() {
            while let Some(Ok(entry)) = root.next() {
                if entry.metadata().is_ok_and(|metadata| metadata.is_file()) {
                    self.blocks
                        .load(&mut self.textures, display, &self.root, entry.path());
                }
            }
        }
    }

    pub fn load_texture<P: AsRef<Path>>(&mut self, display: &WindowDisplay, path: P) {
        self.textures.load(display, path);
    }

    pub fn get_texture_by_id(&self, id: usize) -> Option<&CompressedTexture2d> {
        self.textures.get_by_id(id)
    }

    pub fn get_texture_by_name<I: AsRef<str>>(&self, name: I) -> Option<&CompressedTexture2d> {
        self.textures.get_by_name(name.as_ref())
    }

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

        chunk.get_block(position)
    }

    #[must_use]
    pub fn block_exists(&self, position: Vec3) -> bool {
        self.find_chunk(position)
            .is_some_and(|chunk| chunk.check_for_block(position))
    }

    fn compute_chunk_mesh(&self, chunk: &Chunk) -> [Mesh; 6] {
        let origin = chunk.origin.as_vec2();
        let mut meshes = [Mesh::EMPTY; 6];

        for y in 0..(CHUNK_SIZE as u16 * SUBCHUNK_COUNT as u16) {
            for z in 0..(CHUNK_SIZE as u16) {
                for x in 0..(CHUNK_SIZE as u16) {
                    let position = u16vec3(x, y, z);
                    let float_position =
                        position.as_vec3() + (vec3(origin.x, 0.0, origin.y) * CHUNK_SIZE as f32);

                    if chunk.get_block_inner(position).is_some() {
                        let position = position.as_vec3()
                            + (vec3(origin.x, 0.0, origin.y) * CHUNK_SIZE as f32);

                        for face in Face::ALL {
                            if self
                                .find_block(float_position + face.as_normal().as_vec3())
                                .is_none()
                            {
                                let mesh = &mut meshes[face.normal_index()];
                                let vertices = face.as_vertices();
                                let vertice_corners = face.as_vertice_corners();

                                mesh.vertices.extend([0, 1, 2, 2, 3, 0].map(|vertice| {
                                    let [side1, side2, corner] = vertice_corners[vertice]
                                        .get_neighbours(face)
                                        .map(|neighbour| {
                                            self.block_exists(position + neighbour.as_vec3())
                                        });

                                    let ambient_occlusion = vertex_ao(side1, side2, corner);

                                    let color: Vec3 = Color::WHITE.as_value();
                                    let color = color * ambient_occlusion;

                                    Vertex {
                                        position: vertices[vertice] + position,
                                        uv: face.as_uv()[vertice],
                                        color: Color::from(color),
                                    }
                                }));
                            }
                        }
                    }
                }
            }
        }

        meshes
    }

    #[must_use]
    pub fn compute_world_mesh(&self) -> Vec<[Mesh; 6]> {
        let mut meshes = Vec::new();

        for chunk in self.chunks.values() {
            meshes.push(self.compute_chunk_mesh(chunk));

            println!(
                "[{:18}] Generated mesh for chunk at {}",
                "INFO/Rendering".bright_green(),
                format!("{:>2} {:>2}", chunk.origin.x, chunk.origin.y)
                    .bright_blue()
                    .bold()
            );
        }

        meshes
    }
}
