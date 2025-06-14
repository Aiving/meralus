use std::{
    collections::HashMap,
    fs::DirEntry,
    ops::Range,
    path::{Path, PathBuf},
};

use glam::{DVec3, IVec2, Mat4, U16Vec3, Vec2, Vec3, ivec3, u16vec3, vec3};
use glium::{
    Texture2d,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler},
};
use meralus_engine::WindowDisplay;
use meralus_shared::Color;
use meralus_world::{
    Axis, CHUNK_SIZE, CHUNK_SIZE_F32, CHUNK_SIZE_U16, Chunk, ChunkManager, Face, SUBCHUNK_COUNT_U16,
};
use owo_colors::OwoColorize;

use crate::{
    Aabb, BakedBlockModelLoader, Block, BlockManager, TextureLoader,
    loaders::BakedBlockModel,
    raycast::{HitType, RayCastResult},
    renderers::Voxel,
    vertex_ao,
};

const GRASS_COLOR: Color = Color::from_hsl(120.0, 0.4, 0.75);

pub struct Game {
    textures: TextureLoader,
    blocks: BlockManager,
    models: BakedBlockModelLoader,
    chunk_manager: ChunkManager,
    players: Vec<Player>,
    root: PathBuf,
}

pub struct Player {
    pub position: Vec3,
    pub nickname: String,
    pub is_me: bool,
}

struct LightNode(U16Vec3, IVec2);

impl LightNode {
    pub const fn get_position(&self) -> U16Vec3 {
        self.0
    }
}

struct BfsLight {
    queue: Vec<LightNode>,
}

impl BfsLight {
    const fn new() -> Self {
        Self { queue: Vec::new() }
    }

    fn push(&mut self, node: LightNode) {
        self.queue.push(node);
    }

    fn calculate(
        &mut self,
        chunk_manager: &mut ChunkManager,
        blocks: &BakedBlockModelLoader,
        is_sky_light: bool,
    ) {
        while let Some(node) = self.queue.pop() {
            if let Some(chunk) = chunk_manager.get_chunk_mut(&node.1) {
                let local_position = node.get_position();
                let world_position = chunk.to_world(local_position);

                let light_level = chunk.get_light(local_position, is_sky_light);

                for face in Face::ALL {
                    let neighbour_pos = world_position + face.as_normal();
                    let neighbour_position = neighbour_pos.as_vec3();

                    if let Some(chunk) =
                        chunk_manager.get_chunk_mut(&ChunkManager::to_local(neighbour_position))
                    {
                        let local_position = chunk.to_local(neighbour_position);

                        if !chunk.contains_local_position(local_position) {
                            continue;
                        }

                        if chunk
                            .get_block_unchecked(local_position)
                            .is_none_or(|block| !blocks.get(block.into()).unwrap().is_opaque())
                            && chunk.get_light(local_position, is_sky_light) + 2 <= light_level
                        {
                            chunk.set_light(
                                local_position,
                                is_sky_light,
                                if is_sky_light && face == Face::Bottom && light_level == 15 {
                                    light_level
                                } else {
                                    light_level - 1
                                },
                            );

                            self.queue.push(LightNode(local_position, chunk.origin));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Colliders {
    pub top: Option<DVec3>,
    pub bottom: Option<DVec3>,
    pub left: Option<DVec3>,
    pub right: Option<DVec3>,
    pub front: Option<DVec3>,
    pub back: Option<DVec3>,
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
            blocks: BlockManager::new(),
            models: BakedBlockModelLoader::default(),
            players: Vec::new(),
            root: root.into(),
            chunk_manager: ChunkManager::from_range(x_range, &z_range),
        }
    }

    pub const fn chunk_manager(&self) -> &ChunkManager {
        &self.chunk_manager
    }

    pub const fn chunk_manager_mut(&mut self) -> &mut ChunkManager {
        &mut self.chunk_manager
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

    pub fn update_block_sky_light(&mut self, position: Vec3) {
        let mut bfs_light = BfsLight::new();

        for face in Face::ALL {
            let position = position + face.as_normal().as_vec3();

            if let Some(chunk) = self
                .chunk_manager
                .get_chunk(&ChunkManager::to_local(position))
            {
                let local = chunk.to_local(position);

                if !chunk.contains_local_position(local) {
                    continue;
                }

                if chunk.get_block_unchecked(local).is_none() {
                    bfs_light.push(LightNode(local, chunk.origin));
                }
            }
        }

        bfs_light.calculate(&mut self.chunk_manager, &self.models, true);
    }

    pub fn generate_lights(&mut self) {
        let mut bfs_light = BfsLight::new();

        for chunk in self.chunk_manager.chunks_mut() {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let position = u16vec3(x as u16, 255, z as u16);

                    if chunk
                        .get_block_unchecked(position)
                        .is_none_or(|block| !self.models.get(block.into()).unwrap().is_opaque())
                    {
                        chunk.set_sky_light(position, 15);

                        bfs_light.push(LightNode(position, chunk.origin));
                    }
                }
            }
        }

        bfs_light.calculate(&mut self.chunk_manager, &self.models, true);
    }

    pub fn set_block_light(&mut self, position: Vec3, light_level: u8) {
        let mut bfs_light = BfsLight::new();

        if let Some(chunk) = self
            .chunk_manager
            .get_chunk_mut(&ChunkManager::to_local(position))
        {
            let position = chunk.to_local(position);

            chunk.set_block_light(position, light_level);

            bfs_light.push(LightNode(position, chunk.origin));
        }

        bfs_light.calculate(&mut self.chunk_manager, &self.models, false);
    }

    pub fn register_block<T: Block + 'static>(&mut self, block: T) {
        let id = block.id();

        self.load_block(self.root.join("models").join(id).with_extension("json"));

        self.blocks.register(block);
    }

    pub fn load_block<P: AsRef<Path>>(&mut self, path: P) {
        self.models
            .load(&mut self.textures, &self.root, path)
            .unwrap();
    }

    pub fn collides(&self, aabb: Aabb) -> bool {
        let min = aabb.min.floor().as_ivec3().to_array();
        let max = aabb.max.ceil().as_ivec3().to_array();

        for y in min[1]..max[1] {
            for z in min[2]..max[2] {
                for x in min[0]..max[0] {
                    let position = ivec3(x, y, z).as_dvec3();

                    if self.chunk_manager.contains_block(position.as_vec3()) {
                        let block = Aabb::new(position, position + DVec3::ONE);

                        if aabb.intersects_with_x(block)
                            && aabb.intersects_with_y(block)
                            && aabb.intersects_with_z(block)
                        {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn get_colliders(&self, collider_position: DVec3, aabb: Aabb) -> Colliders {
        let min = aabb.min.floor().as_ivec3().to_array();
        let max = aabb.max.ceil().as_ivec3().to_array();

        let mut colliders = Colliders::default();

        for y in min[1]..max[1] {
            for z in min[2]..max[2] {
                for x in min[0]..max[0] {
                    let position = ivec3(x, y, z).as_dvec3();

                    if self.chunk_manager.contains_block(position.as_vec3()) {
                        let block = Aabb::new(position, position + DVec3::ONE);

                        if aabb.intersects_with_x(block)
                            && aabb.intersects_with_y(block)
                            && aabb.intersects_with_z(block)
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
        if let Ok(root) = self.root.join("models").read_dir()
            && let Ok(mut root) = root.collect::<Result<Vec<_>, _>>()
        {
            root.sort_by_key(DirEntry::file_name);

            for entry in root {
                if entry.metadata().is_ok_and(|metadata| metadata.is_file())
                    && !entry.file_name().to_string_lossy().starts_with("cuboid")
                {
                    self.models
                        .load(&mut self.textures, &self.root, entry.path())
                        .unwrap();
                }
            }
        }
    }

    pub fn generate_mipmaps(&mut self, level: usize) {
        self.textures.generate_mipmaps(level);
    }

    pub fn load_texture<P: AsRef<Path>>(&mut self, path: P) {
        self.textures.load(path).unwrap();
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

    pub fn get_texture<I: AsRef<str>>(&self, name: I) -> Option<(Vec2, Vec2, u8)> {
        self.textures.get_texture(name.as_ref())
    }

    fn raycast_into(position: Vec3, start: DVec3, end: DVec3, aabb: Aabb) -> Option<RayCastResult> {
        aabb.calculate_intercept(start - position.as_dvec3(), end - position.as_dvec3())
            .map(|raytraceresult| {
                RayCastResult::new3(
                    raytraceresult.hit_vec + position.as_dvec3(),
                    raytraceresult.hit_side,
                    position,
                )
            })
    }

    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub fn raycast(
        &self,
        mut origin: DVec3,
        target: DVec3,
        last_uncollidable_block: bool,
    ) -> Option<RayCastResult> {
        if origin.is_nan() || target.is_nan() {
            None
        } else {
            let mut start = origin.floor();
            let end = target.floor();

            let mut position = start.as_vec3();
            let block = self.get_model_for(position);

            if let Some(block) = block {
                let result =
                    Self::raycast_into(position, origin, target, Aabb::from(block.bounding_box));

                if result.is_some() {
                    return result;
                }
            }

            let mut result: Option<RayCastResult> = None;

            for _ in 0..200 {
                if origin.is_nan() {
                    return None;
                }

                if (start.x - end.x).abs() < 0.0001
                    && (start.y - end.y).abs() < 0.0001
                    && (start.z - end.z).abs() < 0.0001
                {
                    return if last_uncollidable_block {
                        result
                    } else {
                        None
                    };
                }

                let mut modify_d3 = true;
                let mut modify_d4 = true;
                let mut modify_d5 = true;

                let mut d0 = 999.0f64;
                let mut d1 = 999.0f64;
                let mut d2 = 999.0f64;

                if end.x > start.x {
                    d0 = start.x + 1.0;
                } else if end.x < start.x {
                    d0 = start.x + 0.0;
                } else {
                    modify_d3 = false;
                }

                if end.y > start.y {
                    d1 = start.y + 1.0;
                } else if end.y < start.y {
                    d1 = start.y + 0.0;
                } else {
                    modify_d4 = false;
                }

                if end.z > start.z {
                    d2 = start.z + 1.0;
                } else if end.z < start.z {
                    d2 = start.z + 0.0;
                } else {
                    modify_d5 = false;
                }

                let mut d3 = 999.0f64;
                let mut d4 = 999.0f64;
                let mut d5 = 999.0f64;

                let d6 = target.x - origin.x;
                let d7 = target.y - origin.y;
                let d8 = target.z - origin.z;

                if modify_d3 {
                    d3 = (d0 - origin.x) / d6;
                }

                if modify_d4 {
                    d4 = (d1 - origin.y) / d7;
                }

                if modify_d5 {
                    d5 = (d2 - origin.z) / d8;
                }

                if d3 == -0.0 {
                    d3 = -0.0001;
                }

                if d4 == -0.0 {
                    d4 = -0.0001;
                }

                if d5 == -0.0 {
                    d5 = -0.0001;
                }

                let facing_at = if d3 < d4 && d3 < d5 {
                    origin = DVec3::new(d0, d7.mul_add(d3, origin.y), d8.mul_add(d3, origin.z));

                    if end.x > start.x {
                        Face::Left
                    } else {
                        Face::Right
                    }
                } else if d4 < d5 {
                    origin = DVec3::new(d6.mul_add(d4, origin.x), d1, d8.mul_add(d4, origin.z));

                    if end.y > start.y {
                        Face::Bottom
                    } else {
                        Face::Top
                    }
                } else {
                    origin = DVec3::new(d6.mul_add(d5, origin.x), d7.mul_add(d5, origin.y), d2);

                    if end.z > start.z {
                        Face::Front
                    } else {
                        Face::Back
                    }
                };

                start = origin.floor()
                    - match facing_at {
                        Face::Right => DVec3::X,
                        Face::Top => DVec3::Y,
                        Face::Back => DVec3::Z,
                        Face::Bottom | Face::Left | Face::Front => DVec3::ZERO,
                    };

                position = start.as_vec3();

                let block = self.get_model_for(position);

                if let Some(block) = block {
                    let result = Self::raycast_into(
                        position,
                        origin,
                        target,
                        Aabb::from(block.bounding_box),
                    );

                    if result.is_some() {
                        return result;
                    }
                } else {
                    result.replace(RayCastResult::new(
                        HitType::None,
                        origin,
                        facing_at,
                        position,
                    ));
                }
            }

            if last_uncollidable_block {
                result
            } else {
                None
            }
        }
    }

    pub fn compute_chunk_mesh_at(&self, position: &IVec2) -> Option<[(Face, [Vec<Voxel>; 2]); 6]> {
        self.chunk_manager
            .get_chunk(position)
            .map(|chunk| self.compute_chunk_mesh(chunk))
    }

    pub fn get_model_for(&self, position: Vec3) -> Option<&BakedBlockModel> {
        self.chunk_manager
            .get_block(position)
            .and_then(|block| self.models.get(block.into()))
    }

    #[allow(clippy::too_many_lines)]
    pub fn compute_chunk_mesh(&self, chunk: &Chunk) -> [(Face, [Vec<Voxel>; 2]); 6] {
        let origin = chunk.origin.as_vec2();
        let mut voxels = Face::ALL.map(|face| (face, [const { Vec::new() }; 2]));

        for y in 0..(CHUNK_SIZE_U16 * SUBCHUNK_COUNT_U16) {
            for z in 0..CHUNK_SIZE_U16 {
                for x in 0..CHUNK_SIZE_U16 {
                    let local_position = u16vec3(x, y, z);
                    let world_position =
                        local_position.as_vec3() + (vec3(origin.x, 0.0, origin.y) * CHUNK_SIZE_F32);

                    if let Some(model) = chunk
                        .get_block(local_position)
                        .and_then(|block_id| self.models.get(block_id.into()))
                    {
                        let position = local_position.as_vec3()
                            + (vec3(origin.x, 0.0, origin.y) * CHUNK_SIZE_F32);

                        for element in &model.elements {
                            let matrix = element.rotation.map(|rotation| {
                                let angle = rotation.angle.to_radians();

                                let matrix;
                                let mut scale = Vec3::ZERO;

                                match rotation.axis {
                                    Axis::X => {
                                        matrix = Mat4::from_rotation_x(angle);

                                        scale.y = 1.0;
                                        scale.z = 1.0;
                                    }
                                    Axis::Y => {
                                        matrix = Mat4::from_rotation_y(angle);

                                        scale.x = 1.0;
                                        scale.z = 1.0;
                                    }
                                    Axis::Z => {
                                        matrix = Mat4::from_rotation_z(angle);

                                        scale.x = 1.0;
                                        scale.y = 1.0;
                                    }
                                }

                                scale = Vec3::ONE;

                                (matrix, rotation.origin, scale)
                            });

                            for model_face in element.faces.iter().flatten() {
                                let neighbour_position =
                                    world_position + model_face.face.as_normal().as_vec3();

                                let culled = model_face.cull_face.is_some_and(|cull_face| {
                                    let neighbour = self.chunk_manager.get_block(
                                        world_position + cull_face.as_normal().as_vec3(),
                                    );

                                    neighbour
                                        .and_then(|neighbour| self.models.get(neighbour.into()))
                                        .is_some_and(|model| {
                                            if model.is_opaque() {
                                                true
                                            } else {
                                                let opposite_face = cull_face.opposite();

                                                model.elements.iter().any(|element| {
                                                    element.faces[opposite_face.normal_index()]
                                                        .as_ref()
                                                        .is_some_and(|face| {
                                                            if face.is_opaque {
                                                                true
                                                            } else {
                                                                face.uv.eq(&model_face.uv)
                                                            }
                                                        })
                                                })
                                            }
                                        })
                                });

                                if !culled {
                                    let mut vertices =
                                        model_face.face.as_vertices().map(|vertice| {
                                            Vec3::from_array(element.cube.origin.to_array())
                                                + vertice
                                                    * Vec3::from_array(element.cube.size.to_array())
                                        });

                                    let mut uvs = model_face.face.as_uv();

                                    let mut aos =
                                        model_face.face.as_vertice_corners().map(|corner| {
                                            let [side1, side2, corner] = corner
                                                .get_neighbours(model_face.face)
                                                .map(|neighbour| {
                                                    self.chunk_manager
                                                        .get_block(position + neighbour.as_vec3())
                                                        .is_some_and(|block| {
                                                            self.models
                                                                .get(block.into())
                                                                .unwrap()
                                                                .ambient_occlusion
                                                        })
                                                });

                                            vertex_ao(side1, side2, corner)
                                        });

                                    // let mut aos_flipped = false;

                                    if aos[1] + aos[2] > aos[0] + aos[3] {
                                        // aos_flipped = true;

                                        // aos = aos[1], aos[2], aos[3], aos[0]

                                        vertices.swap(0, 1);
                                        vertices.swap(1, 2);
                                        vertices.swap(2, 3);

                                        aos.swap(0, 1);
                                        aos.swap(1, 2);
                                        aos.swap(2, 3);

                                        uvs.swap(0, 1);
                                        uvs.swap(1, 2);
                                        uvs.swap(2, 3);
                                    }

                                    let vertices =
                                        matrix.map_or(vertices, |(matrix, origin, scale)| {
                                            vertices.map(|vertice| {
                                                matrix.transform_point3(vertice - origin) * scale
                                                    + origin
                                            })
                                        });

                                    let voxels = &mut voxels[model_face.face.normal_index()].1;
                                    let voxels = if model_face.is_opaque {
                                        &mut voxels[0]
                                    } else {
                                        &mut voxels[1]
                                    };

                                    voxels.push(Voxel {
                                        position,
                                        vertices,
                                        face: model_face.face,
                                        origin: chunk.origin,
                                        aos,
                                        light: self.chunk_manager.get_light(neighbour_position),
                                        color: if model.name == "grass_block" && model_face.tint {
                                            GRASS_COLOR
                                        } else {
                                            Color::WHITE
                                        },
                                        uvs: uvs.map(|uv| {
                                            model_face.uv.offset + uv * model_face.uv.scale
                                        }),
                                        is_opaque: model_face.is_opaque,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        voxels
    }

    #[must_use]
    pub fn compute_world_mesh(&self) -> HashMap<(IVec2, Face), [Vec<Voxel>; 2]> {
        let mut meshes = HashMap::new();

        for chunk in self.chunk_manager.chunks() {
            for (face, data) in self.compute_chunk_mesh(chunk) {
                meshes.insert((chunk.origin, face), data);
            }

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
