use crate::{BakedBlockModelLoader, TextureLoader, mesh::Mesh, vertex_ao};
use glam::{IVec2, IVec3, U16Vec3, Vec2, Vec3, ivec3, u16vec3, vec3};
use meralus_engine::{
    Vertex, WindowDisplay,
    glium::{
        Texture2d,
        uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler},
    },
};
use meralus_shared::Color;
use meralus_world::{CHUNK_SIZE, Chunk, ChunkManager, Face, SUBCHUNK_COUNT};
use owo_colors::OwoColorize;
use std::{
    ops::Range,
    path::{Path, PathBuf},
};

pub struct Game {
    textures: TextureLoader,
    blocks: BakedBlockModelLoader,
    pub chunk_manager: ChunkManager,
    players: Vec<Player>,
    root: PathBuf,
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

struct LightNode(U16Vec3, IVec2);

impl LightNode {
    pub const fn get_position(&self) -> U16Vec3 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub const fn get_center(&self, size: Vec3) -> Vec3 {
        vec3(
            self.min.x + size.x / 2.0,
            self.min.y + size.y / 2.0,
            self.min.z + size.z / 2.0,
        )
    }

    pub const fn intersects_x(&self, against: Self) -> bool {
        self.min.x < against.max.x && self.max.x > against.min.x
    }

    pub const fn intersects_y(&self, against: Self) -> bool {
        self.min.y < against.max.y && self.max.y > against.min.y
    }

    pub const fn intersects_z(&self, against: Self) -> bool {
        self.min.z < against.max.z && self.max.z > against.min.z
    }

    pub const fn get_clip_x(&self, against: Self, mut delta_x: f32) -> f32 {
        if self.intersects_y(against) && self.intersects_z(against) {
            if delta_x > 0.0 && self.max.x <= against.min.x {
                let clip = against.min.x - self.max.x;

                if delta_x > clip {
                    delta_x = clip;
                }
            }

            if delta_x < 0.0 && self.min.x >= against.max.x {
                let clip = against.max.x - self.min.x;

                if delta_x < clip {
                    delta_x = clip;
                }
            }
        }

        delta_x
    }

    pub const fn get_clip_y(&self, against: Self, mut delta_y: f32) -> f32 {
        if self.intersects_x(against) && self.intersects_z(against) {
            if delta_y > 0.0 && self.max.y <= against.min.y {
                let clip = against.min.y - self.max.y;

                if delta_y > clip {
                    delta_y = clip;
                }
            }

            if delta_y < 0.0 && self.min.y >= against.max.y {
                let clip = against.max.y - self.min.y;

                if delta_y < clip {
                    delta_y = clip;
                }
            }
        }

        delta_y
    }

    pub const fn get_clip_z(&self, against: Self, mut delta_z: f32) -> f32 {
        if self.intersects_x(against) && self.intersects_y(against) {
            if delta_z > 0.0 && self.max.z <= against.min.z {
                let clip = against.min.z - self.max.z;

                if delta_z > clip {
                    delta_z = clip;
                }
            }

            if delta_z < 0.0 && self.min.z >= against.max.z {
                let clip = against.max.z - self.min.z;

                if delta_z < clip {
                    delta_z = clip;
                }
            }
        }

        delta_z
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Colliders {
    pub top: Option<Vec3>,
    pub bottom: Option<Vec3>,
    pub left: Option<Vec3>,
    pub right: Option<Vec3>,
    pub front: Option<Vec3>,
    pub back: Option<Vec3>,
}

impl Game {
    #[must_use]
    pub fn new(
        display: &WindowDisplay,
        root: impl Into<PathBuf>,
        x_range: Range<i32>,
        z_range: Range<i32>,
    ) -> Self {
        Self {
            textures: TextureLoader::new(display),
            blocks: BakedBlockModelLoader::default(),
            players: Vec::new(),
            root: root.into(),
            chunk_manager: ChunkManager::from_range(x_range, &z_range),
        }
    }

    pub fn generate_world(&mut self, seed: u32) {
        self.chunk_manager.generate_surface(seed);
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn set_block_light(&mut self, position: Vec3, light_level: u8) {
        let mut lights_bfs_queue = Vec::new();

        if let Some(chunk) = self
            .chunk_manager
            .get_chunk_mut(&ChunkManager::to_local(position))
        {
            let position = chunk.to_local(position);

            chunk.set_block_light(position, light_level);

            lights_bfs_queue.push(LightNode(position, chunk.origin));
        }

        while let Some(node) = lights_bfs_queue.pop() {
            if let Some(chunk) = self.chunk_manager.get_chunk_mut(&node.1) {
                let local_position = node.get_position();
                let world_position = chunk.to_world(local_position);

                let light_level = chunk.get_block_light(local_position);

                for face in Face::ALL {
                    let neighbour_position = (world_position + face.as_normal()).as_vec3();

                    if let Some(chunk) = self
                        .chunk_manager
                        .get_chunk_mut(&ChunkManager::to_local(neighbour_position))
                    {
                        let local_position = chunk.to_local(neighbour_position);

                        if chunk.get_block_unchecked(local_position).is_none()
                            && chunk.get_block_light(local_position) + 2 <= light_level
                        {
                            chunk.set_block_light(local_position, light_level - 1);

                            lights_bfs_queue.push(LightNode(local_position, chunk.origin));
                        }
                    }
                }
            }
        }
    }

    pub fn load_block<P: AsRef<Path>>(&mut self, path: P) {
        self.blocks.load(&mut self.textures, &self.root, path);
    }

    pub fn collides(&self, aabb: Aabb) -> bool {
        let min = aabb.min.floor().as_ivec3().to_array();
        let max = aabb.max.ceil().as_ivec3().to_array();

        for y in min[1]..max[1] {
            for z in min[2]..max[2] {
                for x in min[0]..max[0] {
                    let position = ivec3(x, y, z).as_vec3();

                    if self.chunk_manager.contains_block(position) {
                        let block = Aabb::new(position, position + Vec3::ONE);

                        if aabb.intersects_x(block)
                            && aabb.intersects_y(block)
                            && aabb.intersects_z(block)
                        {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn get_colliders(&self, collider_position: Vec3, aabb: Aabb) -> Colliders {
        let min = aabb.min.floor().as_ivec3().to_array();
        let max = aabb.max.ceil().as_ivec3().to_array();

        let mut colliders = Colliders::default();

        for y in min[1]..max[1] {
            for z in min[2]..max[2] {
                for x in min[0]..max[0] {
                    let position = ivec3(x, y, z).as_vec3();

                    if self.chunk_manager.contains_block(position) {
                        let block = Aabb::new(position, position + Vec3::ONE);

                        if aabb.intersects_x(block)
                            && aabb.intersects_y(block)
                            && aabb.intersects_z(block)
                        {
                            let colliding_position = position - collider_position.floor();

                            if colliding_position.x < 0.0 {
                                colliders.left = Some(position);
                            } else if colliding_position.x > 0.0 {
                                colliders.right = Some(position);
                            } else if colliding_position.y < 0.0 {
                                colliders.bottom = Some(position);
                            } else if colliding_position.y > 0.0 {
                                colliders.top = Some(position);
                            } else if colliding_position.z < 0.0 {
                                colliders.back = Some(position);
                            } else if colliding_position.z > 0.0 {
                                colliders.front = Some(position);
                            }
                        }
                    }
                }
            }
        }

        colliders
    }

    pub fn load_buitlin_blocks(&mut self) {
        if let Ok(mut root) = self.root.join("models").read_dir() {
            while let Some(Ok(entry)) = root.next() {
                if entry.metadata().is_ok_and(|metadata| metadata.is_file()) {
                    self.blocks
                        .load(&mut self.textures, &self.root, entry.path());
                }
            }
        }
    }

    pub fn generate_mipmaps(&mut self, level: usize) {
        self.textures.generate_mipmaps(level);
    }

    pub fn load_texture<P: AsRef<Path>>(&mut self, path: P) {
        self.textures.load(path);
    }

    pub const fn get_texture_atlas(&self) -> &Texture2d {
        self.textures.get_atlas()
    }

    pub fn get_texture_atlas_sampled(&self) -> Sampler<'_, Texture2d> {
        self.textures
            .get_atlas()
            .sampled()
            .minify_filter(MinifySamplerFilter::NearestMipmapLinear)
            .magnify_filter(MagnifySamplerFilter::Nearest)
    }

    pub fn get_texture_count(&self) -> usize {
        self.textures.get_texture_count()
    }

    pub fn get_texture<I: AsRef<str>>(&self, name: I) -> Option<(Vec2, Vec2)> {
        self.textures.get_texture(name.as_ref())
    }

    fn compute_chunk_mesh(&self, chunk: &Chunk) -> [Mesh; 6] {
        let origin = chunk.origin.as_vec2();
        let mut meshes = [Mesh::EMPTY; 6];

        for mesh in &mut meshes {
            mesh.origin = chunk.origin;
        }

        for y in 0..(CHUNK_SIZE as u16 * SUBCHUNK_COUNT as u16) {
            for z in 0..(CHUNK_SIZE as u16) {
                for x in 0..(CHUNK_SIZE as u16) {
                    let local_position = u16vec3(x, y, z);
                    let float_position = local_position.as_vec3()
                        + (vec3(origin.x, 0.0, origin.y) * CHUNK_SIZE as f32);

                    if let Some(model) = chunk
                        .get_block(local_position)
                        .and_then(|block_id| self.blocks.get(block_id.into()))
                    {
                        let position = local_position.as_vec3()
                            + (vec3(origin.x, 0.0, origin.y) * CHUNK_SIZE as f32);

                        for model_face in &model.faces {
                            let neighbour_position =
                                float_position + model_face.face.as_normal().as_vec3();

                            if self.chunk_manager.get_block(neighbour_position).is_none() {
                                let mesh = &mut meshes[model_face.face.normal_index()];

                                let mut vertices = model_face.face.as_bool_vertices();
                                let mut uvs = model_face.uv;
                                let mut overlay_uvs = model_face.overlay_uv.unwrap_or_default();

                                let mut aos = model_face.face.as_vertice_corners().map(|corner| {
                                    let [side1, side2, corner] =
                                        corner.get_neighbours(model_face.face).map(|neighbour| {
                                            self.chunk_manager
                                                .contains_block(position + neighbour.as_vec3())
                                        });

                                    vertex_ao(side1, side2, corner)
                                });

                                if aos[1] + aos[2] > aos[0] + aos[3] {
                                    vertices.swap(0, 1);
                                    vertices.swap(1, 2);
                                    vertices.swap(2, 3);

                                    aos.swap(0, 1);
                                    aos.swap(1, 2);
                                    aos.swap(2, 3);

                                    overlay_uvs.swap(0, 1);
                                    overlay_uvs.swap(1, 2);
                                    overlay_uvs.swap(2, 3);

                                    uvs.swap(0, 1);
                                    uvs.swap(1, 2);
                                    uvs.swap(2, 3);
                                }

                                mesh.vertices.extend([0, 1, 2, 2, 3, 0].map(|vertice| {
                                    let light_level =
                                        self.chunk_manager.get_block_light(neighbour_position);

                                    Vertex::from_vec(
                                        vertices[vertice],
                                        local_position,
                                        uvs[vertice],
                                        if model.name == "grass_block" && model_face.tint {
                                            Color::LIGHT_GREEN
                                        } else {
                                            Color::WHITE
                                        }
                                        .multiply_rgb(model_face.face.get_light_level())
                                        .multiply_rgb(f32::from(light_level + 1) / 16.0)
                                        .multiply_rgb(aos[vertice]),
                                    )
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

        for chunk in self.chunk_manager.chunks() {
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
