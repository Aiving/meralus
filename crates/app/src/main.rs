#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal
)]

use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    ops::Range,
    path::{Path, PathBuf},
    vec,
};

use macroquad::{
    color::hsl_to_rgb,
    miniquad::{gl, window::screen_size},
    prelude::*,
};
use meralus_meshing::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk};

const MOVE_SPEED: f32 = 4.;
const LOOK_SPEED: f32 = 0.1;

fn conf() -> Conf {
    Conf {
        window_title: String::from("Macroquad"),
        window_width: 1260,
        window_height: 768,
        ..Default::default()
    }
}

struct PlayerController {
    position: Vec3,
    // START CAMERA
    yaw: f32,
    pitch: f32,
    front: Vec3,
    right: Vec3,
    up: Vec3,
    // END CAMERA
    velocity: Vec3,
}

fn get_movement_direction() -> Vec3 {
    let mut direction = Vec3::ZERO;

    if is_key_down(KeyCode::W) {
        direction.z += 1.;
    }

    if is_key_down(KeyCode::S) {
        direction.z -= 1.;
    }

    if is_key_down(KeyCode::A) {
        direction.x -= 1.;
    }

    if is_key_down(KeyCode::D) {
        direction.x += 1.;
    }

    direction
}

fn calc_rotation_dirs(yaw: f32, pitch: f32) -> (Vec3, Vec3, Vec3) {
    let front = vec3(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    )
    .normalize();

    let right = front.cross(Vec3::Y).normalize();

    (front, right, right.cross(front).normalize())
}

impl PlayerController {
    const GRAVITY: f32 = 9.81;

    fn is_on_ground(&self, game: &Game) -> bool {
        game.find_block(self.position - vec3(0.0, 2.0, 0.0))
            .is_some()
    }

    fn handle_physics(&mut self, game: &Game, delta: f32) {
        let direction = get_movement_direction();

        let (front, right, _) = calc_rotation_dirs(self.yaw, 0.0);

        let velocity = ((front * direction.z) + (right * direction.x))
            * if is_key_down(KeyCode::LeftControl) && direction.z > 0.0 {
                MOVE_SPEED * 1.5
            } else {
                MOVE_SPEED
            };

        self.velocity.x = velocity.x;
        self.velocity.z = velocity.z;

        if !self.is_on_ground(game) {
            self.velocity.y -= Self::GRAVITY * delta;
        }

        if self.is_on_ground(game) && self.velocity.y <= 0.0 {
            self.velocity.y = 0.0;
        }

        // if is_key_pressed(KeyCode::Space)
        // /* && self.is_on_ground(game) */
        // {
        //     if is_key_down(KeyCode::LeftShift) {
        //         self.velocity.y -= 5.0;
        //     } else {
        //         self.velocity.y = 5.0;
        //     }
        // }

        if is_key_pressed(KeyCode::Space) && self.is_on_ground(game) {
            self.velocity.y = 5.0;
        }

        self.move_and_collide(game, delta);
    }

    fn move_and_collide(&mut self, game: &Game, delta: f32) {
        let position = self.position + (self.velocity * delta);

        if game.find_chunk(position).is_some() {
            let have_under_block = game.find_block(position - Vec3::Y).is_some();

            if !have_under_block {
                self.position.y = position.y;
            }

            self.position.x = position.x;
            self.position.z = position.z;
        }
    }

    fn handle_mouse(&mut self, mouse_delta: Vec2, delta: f32) {
        self.yaw += mouse_delta.x * delta * LOOK_SPEED;
        self.pitch += mouse_delta.y * delta * -LOOK_SPEED;

        self.pitch = if self.pitch > 1.5 { 1.5 } else { self.pitch };
        self.pitch = if self.pitch < -1.5 { -1.5 } else { self.pitch };

        self.front = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize();

        self.right = self.front.cross(Vec3::Y).normalize();
        self.up = self.right.cross(self.front).normalize();
    }
}

impl Default for PlayerController {
    fn default() -> Self {
        let yaw: f32 = 0.0;
        let pitch: f32 = 0.0;

        let front = vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalize();

        let right = front.cross(Vec3::Y).normalize();
        let up = right.cross(front).normalize();

        Self {
            position: Vec3::Y,
            yaw,
            pitch,
            front,
            right,
            up,
            velocity: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Front,
    Back,
}

enum Axis {
    X,
    Y,
    Z,
}

impl Face {
    const ALL: [Self; 6] = [
        Self::Top,
        Self::Bottom,
        Self::Left,
        Self::Right,
        Self::Front,
        Self::Back,
    ];

    fn from_axis_value(axis: Axis, value: f32) -> Self {
        match (axis, value) {
            (Axis::X, 1.0) => Self::Right,
            (Axis::X, 0.0) => Self::Left,
            (Axis::Y, 1.0) => Self::Top,
            (Axis::Y, 0.0) => Self::Bottom,
            (Axis::Z, 1.0) => Self::Front,
            (Axis::Z, 0.0) => Self::Back,
            _ => unreachable!(),
        }
    }
}

fn get_vertice_neighbours(
    block_position: Vec3,
    positive_y: bool,
    positive_x: bool,
    positive_z: bool,
) -> [Vec3; 3] {
    match [positive_x, positive_y, positive_z] {
        // RIGHT TOP    FRONT
        [true, true, true] => [
            block_position + vec3(0.0, 1.0, 1.0),
            block_position + vec3(1.0, 1.0, 0.0),
            block_position + vec3(1.0, 1.0, 1.0),
        ],
        // RIGHT TOP    BACK
        [true, true, false] => [
            block_position + vec3(1.0, 1.0, 0.0),
            block_position + vec3(0.0, 1.0, -1.0),
            block_position + vec3(1.0, 1.0, -1.0),
        ],
        // RIGHT BOTTOM FRONT
        [true, false, true] => [
            block_position + vec3(1.0, -1.0, 0.0),
            block_position + vec3(0.0, -1.0, 1.0),
            block_position + vec3(1.0, -1.0, 1.0),
        ],
        // RIGHT BOTTOM BACK
        [true, false, false] => [
            block_position - vec3(0.0, 1.0, 1.0),
            block_position - vec3(-1.0, 1.0, 0.0),
            block_position - vec3(-1.0, 1.0, 1.0),
        ],
        // LEFT  TOP    FRONT
        [false, true, true] => [
            block_position + vec3(0.0, 1.0, 1.0),
            block_position + vec3(-1.0, 1.0, 0.0),
            block_position + vec3(-1.0, 1.0, 1.0),
        ],
        // LEFT  TOP    BACK
        [false, true, false] => [
            block_position + vec3(-1.0, 1.0, 0.0),
            block_position + vec3(0.0, 1.0, -1.0),
            block_position + vec3(-1.0, 1.0, -1.0),
        ],
        // LEFT  BOTTOM FRONT
        [false, false, true] => [
            block_position - vec3(1.0, 1.0, 0.0),
            block_position - vec3(0.0, 1.0, -1.0),
            block_position - vec3(1.0, 1.0, -1.0),
        ],
        // LEFT  BOTTOM BACK
        [false, false, false] => [
            block_position - vec3(1.0, 1.0, 0.0),
            block_position - vec3(0.0, 1.0, 1.0),
            block_position - vec3(1.0, 1.0, 1.0),
        ],
    }
}

impl Face {
    const VERTICES: [Vec3; 8] = [
        vec3(0.0, 0.0, 1.0), // 0 LEFT  BOTTOM FRONT
        vec3(1.0, 0.0, 1.0), // 1 RIGHT BOTTOM FRONT
        vec3(0.0, 1.0, 1.0), // 2 LEFT  TOP    FRONT
        vec3(1.0, 1.0, 1.0), // 3 RIGHT TOP    FRONT
        vec3(0.0, 0.0, 0.0), // 4 LEFT  BOTTOM BACK
        vec3(1.0, 0.0, 0.0), // 5 RIGHT BOTTOM BACK
        vec3(0.0, 1.0, 0.0), // 6 LEFT  TOP    BACK
        vec3(1.0, 1.0, 0.0), // 7 RIGHT TOP    BACK
    ];

    const fn as_vertices(self) -> [Vec3; 4] {
        match self {
            Self::Top => [
                Self::VERTICES[2],
                Self::VERTICES[6],
                Self::VERTICES[7],
                Self::VERTICES[3],
            ],
            Self::Bottom => [
                Self::VERTICES[1],
                Self::VERTICES[5],
                Self::VERTICES[4],
                Self::VERTICES[0],
            ],
            Self::Left => [
                Self::VERTICES[4],
                Self::VERTICES[6],
                Self::VERTICES[2],
                Self::VERTICES[0],
            ],
            Self::Right => [
                Self::VERTICES[1],
                Self::VERTICES[3],
                Self::VERTICES[7],
                Self::VERTICES[5],
            ],
            Self::Front => [
                Self::VERTICES[1],
                Self::VERTICES[3],
                Self::VERTICES[2],
                Self::VERTICES[0],
            ],
            Self::Back => [
                Self::VERTICES[5],
                Self::VERTICES[7],
                Self::VERTICES[6],
                Self::VERTICES[4],
            ],
        }
    }

    const fn as_uv(self) -> [Vec2; 4] {
        match self {
            Self::Top => [
                vec2(0.0, 0.0),
                vec2(1.0, 0.0),
                vec2(1.0, 1.0),
                vec2(0.0, 1.0),
            ],
            Self::Bottom => [
                vec2(0.0, 1.0),
                vec2(1.0, 1.0),
                vec2(1.0, 0.0),
                vec2(0.0, 0.0),
            ],
            Self::Left | Self::Right | Self::Front | Self::Back => [
                vec2(0.0, 1.0),
                vec2(0.0, 0.0),
                vec2(1.0, 0.0),
                vec2(1.0, 1.0),
            ],
        }
    }

    const fn as_normal(self) -> Vec3 {
        match self {
            Self::Top => Vec3::Y,
            Self::Bottom => Vec3::NEG_Y,
            Self::Right => Vec3::X,
            Self::Left => Vec3::NEG_X,
            Self::Front => Vec3::Z,
            Self::Back => Vec3::NEG_Z,
        }
    }
}

trait VertexExt {
    fn with_normal(self, normal: Vec4) -> Self;
}

trait Vec3Ext {
    fn as_color(&self) -> Color;
}

impl VertexExt for Vertex {
    fn with_normal(mut self, normal: Vec4) -> Self {
        self.normal = normal;

        self
    }
}

impl Vec3Ext for Vec3 {
    fn as_color(&self) -> Color {
        for (pos, vertice) in Face::VERTICES.iter().enumerate() {
            if self == vertice {
                return hsl_to_rgb(pos as f32 / 8.0, 1.0, 0.5);
            }
        }

        BLACK
    }
}

struct Block {
    faces: Vec<(Face, Texture2D, Option<Color>)>,
}

impl FromIterator<(Face, Texture2D, Option<Color>)> for Block {
    fn from_iter<I: IntoIterator<Item = (Face, Texture2D, Option<Color>)>>(iter: I) -> Self {
        let faces = iter.into_iter().collect();

        Self { faces }
    }
}

impl Block {
    fn get_face_textures(&self, face: Face) -> Vec<(Texture2D, Option<Color>)> {
        self.faces
            .iter()
            .filter_map(|(f, t, c)| {
                if *f == face {
                    Some((t.clone(), *c))
                } else {
                    None
                }
            })
            .collect()
    }
}

struct Game {
    textures: HashMap<String, Texture2D>,
    blocks: Vec<Block>,
    chunks: HashMap<IVec3, Chunk>,
}

const AMBIENT_OCCLUSION_VALUES: [f32; 4] = [0.1, 0.25, 0.5, 1.0];

fn vertex_ao(side1: bool, side2: bool, corner: bool) -> f32 {
    AMBIENT_OCCLUSION_VALUES[if side1 && side2 {
        0
    } else {
        3 - (usize::from(side1) + usize::from(side2) + usize::from(corner))
    }]
}

struct BackedFace {
    position: IVec3,
    face: Face,
    mesh: Mesh,
    // ambient_occlusion: f32
}

trait CameraExt {
    fn unproject_position(&self, position: Vec3) -> Option<(Vec2, f32)>;
}

impl CameraExt for Camera3D {
    fn unproject_position(&self, position: Vec3) -> Option<(Vec2, f32)> {
        let (width, height) = screen_size();
        let clip_space = self.matrix() * position.extend(1.0);

        if clip_space.w <= 0.0 {
            return None;
        }

        let ndc = clip_space.truncate() / clip_space.w;

        let x = (ndc.x + 1.0) * 0.5 * width;
        let y = (1.0 - ndc.y) * 0.5 * height;

        Some((vec2(x, y), clip_space.w))
    }
}

impl Game {
    fn new(seed: u32, x_range: Range<i32>, z_range: Range<i32>) -> Self {
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

    fn load_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    fn load_texture<I: Into<String>, P: AsRef<Path>>(&mut self, id: I, path: P) {
        let id: String = id.into();

        if let Entry::Vacant(entry) = self.textures.entry(id) {
            if let Ok(data) = fs::read(path) {
                let texture = Texture2D::from_file_with_format(&data, None);

                entry.insert(texture);
            }
        }
    }

    fn get_texture<I: AsRef<str>>(&self, id: I) -> Option<Texture2D> {
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
    fn find_chunk(&self, position: Vec3) -> Option<&Chunk> {
        self.chunks.get(&ivec3(
            position.x.floor() as i32 >> 4,
            0,
            position.z.floor() as i32 >> 4,
        ))
    }

    fn find_chunk_mut(&mut self, position: Vec3) -> Option<&mut Chunk> {
        self.chunks.get_mut(
            &vec3(
                (position.x * (1.0 / CHUNK_SIZE as f32)).floor(),
                0.0,
                (position.z * (1.0 / CHUNK_SIZE as f32)).floor(),
            )
            .as_ivec3(),
        )
    }

    fn find_block(&self, position: Vec3) -> Option<u8> {
        let chunk = self.find_chunk(position)?;

        chunk.get_block_unchecked(position)
    }

    fn block_exists(&self, position: Vec3) -> bool {
        self.find_chunk(position)
            .is_some_and(|chunk| chunk.check_for_block(position))
    }

    fn get_face_mesh(
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
                let [side1, side2, corner] = get_vertice_neighbours(
                    position,
                    vertices[i].y > 0.0,
                    vertices[i].x > 0.0,
                    vertices[i].z > 0.0,
                )
                .map(|pos| self.find_block(pos).is_some());

                let ambient_occlusion = vertex_ao(side1, side2, corner);

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

    fn bake_face(
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

fn dirt_block(game: &Game) -> Block {
    let dirt = game.get_texture("dirt").unwrap();

    Block::from_iter([
        (Face::Top, dirt.weak_clone(), None),
        (Face::Bottom, dirt.weak_clone(), None),
        (Face::Right, dirt.weak_clone(), None),
        (Face::Left, dirt.weak_clone(), None),
        (Face::Front, dirt.weak_clone(), None),
        (Face::Back, dirt.weak_clone(), None),
    ])
}

fn grass_block(game: &Game) -> Block {
    let dirt = game.get_texture("dirt").unwrap();
    let top = game.get_texture("grass-block/top").unwrap();
    let side = game.get_texture("grass-block/side").unwrap();
    let overlay = game.get_texture("grass-block/side-overlay").unwrap();

    let color = Color::from_hex(0x5fe366);

    Block::from_iter([
        (Face::Top, top.weak_clone(), Some(color)),
        (Face::Bottom, dirt.weak_clone(), None),
        (Face::Right, side.weak_clone(), None),
        (Face::Left, side.weak_clone(), None),
        (Face::Front, side.weak_clone(), None),
        (Face::Back, side.weak_clone(), None),
        (Face::Right, overlay.weak_clone(), Some(color)),
        (Face::Left, overlay.weak_clone(), Some(color)),
        (Face::Front, overlay.weak_clone(), Some(color)),
        (Face::Back, overlay.weak_clone(), Some(color)),
    ])
}

#[tokio::main]
async fn main() {
    // tokio::spawn(async move {
    //     let mut client = BufWriter::new(TcpStream::connect("192.168.1.5:37565").await.unwrap());

    //     let mut encoder = ZlibEncoder::new(Vec::new());

    //     encoder.write_all(b"hello world!").await.unwrap();
    //     encoder.shutdown().await.unwrap();

    //     client.write_all(&encoder.into_inner()).await.unwrap();
    //     client.shutdown().await.unwrap();

    //     println!("W ROT E BAL");
    // });

    macroquad::Window::from_config(conf(), app(Game::new(12723, 0..1, 0..1)));
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::default(),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn set_rotation_x(&mut self, value: f32) {
        self.rotation = Quat::from_axis_angle(Vec3::X, value) * self.rotation;
    }

    pub fn set_rotation_y(&mut self, value: f32) {
        self.rotation = Quat::from_axis_angle(Vec3::Y, value) * self.rotation;
    }

    pub fn set_rotation_z(&mut self, value: f32) {
        self.rotation = Quat::from_axis_angle(Vec3::Z, value) * self.rotation;
    }

    pub fn set_rotation(&mut self, x: f32, y: f32, z: f32) {
        self.set_rotation_x(x);
        self.set_rotation_y(y);
        self.set_rotation_z(z);
    }

    pub const fn set_scale_x(&mut self, value: f32) {
        self.scale.x = value;
    }

    pub const fn set_scale_y(&mut self, value: f32) {
        self.scale.y = value;
    }

    pub const fn set_scale_z(&mut self, value: f32) {
        self.scale.z = value;
    }

    pub const fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        self.set_scale_x(x);
        self.set_scale_y(y);
        self.set_scale_z(z);
    }

    pub fn set_translation_x(&mut self, value: f32) {
        self.translation.x += value;
    }

    pub fn set_translation_y(&mut self, value: f32) {
        self.translation.y += value;
    }

    pub fn set_translation_z(&mut self, value: f32) {
        self.translation.z += value;
    }

    pub fn set_translation(&mut self, x: f32, y: f32, z: f32) {
        self.set_translation_x(x);
        self.set_translation_y(y);
        self.set_translation_z(z);
    }

    #[must_use]
    pub fn affine(&self) -> Affine3A {
        println!("rotation: {}", self.rotation);

        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

struct GameState {
    current_block: Option<(u8, IVec3)>,
}

fn debug_face_vertices(game: &GameState, backed: &BackedFace, vertices: &mut Vec<Vertex>) {
    if let Some((_, block_position)) = &game.current_block {
        if &backed.position == block_position {
            for vertice in &backed.mesh.vertices {
                if !vertices.iter().any(|b| vertice.position == b.position) {
                    vertices.push(*vertice);
                }

                draw_sphere(
                    vertice.position,
                    0.1,
                    None,
                    (vertice.position - backed.position.as_vec3()).as_color(),
                );
            }
        }
    }
}

fn debug_current_block_faces(game: &GameState, vertices: &[Vertex]) {
    if let Some((_, block_position)) = game.current_block {
        let vertices_count = vertices.len() as f32;
        let height = (vertices_count + 2.0) * 12.0;

        draw_rectangle(
            screen_width() - 275.0,
            screen_height() - height,
            275.0,
            height,
            BLACK,
        );

        for (i, vertex) in vertices.iter().enumerate() {
            let position = vertex.position - block_position.as_vec3();
            let text = format!(
                "{:5?} {:6?} {:?} ({}) AO: {}",
                Face::from_axis_value(Axis::X, position.x),
                Face::from_axis_value(Axis::Y, position.y),
                Face::from_axis_value(Axis::Z, position.z),
                position,
                vertex.normal.w
            );
            let i = i as f32;
            let measured = measure_text(&text, None, 16, 1.0);

            draw_text(
                &text,
                screen_width() - measured.width - 12.0,
                12.0f32.mul_add(-i, screen_height() - measured.height),
                16.0,
                (vertex.position - block_position.as_vec3()).as_color(),
            );
        }
    }
}

fn debug_current_block(game: &Game, state: &mut GameState, camera: &Camera3D) {
    if let Some((_, block_position)) = state.current_block {
        for face in Face::ALL {
            let normal = block_position.as_vec3() + (face.as_normal() / 2.0) + (Vec3::ONE / 2.0);

            // for vertice in &backed.mesh.vertices {
            //     if let Some(position) = camera.unproject_position(vertice.position) {
            //         let text = vertice.position.to_string();
            //         let measured = measure_text(&text, None, 16, 1.0);

            //         draw_text(
            //             &text,
            //             position.x - (measured.width / 2.),
            //             position.y,
            //             16.0,
            //             BLUE,
            //         );
            //     }
            // }

            if let Some((position, w)) = camera.unproject_position(normal) {
                let text = format!("{face:?}");
                let measured = measure_text(&text, None, 16, 1.0);

                draw_text(
                    &text,
                    position.x - (measured.width / 2.),
                    position.y,
                    16.0,
                    Color::from_vec((WHITE.to_vec().xyz() * ((15.0 - w) / 15.0)).extend(1.0)),
                );
            }
        }

        if is_key_down(KeyCode::LeftControl) && is_key_pressed(KeyCode::F) {
            state.current_block.take();
        } else {
            show_block_info(game, state, block_position.as_vec3());
        }
    }
}

fn show_block_info(game: &Game, state: &mut GameState, position: Vec3) {
    if let Some(block) = game.find_block(position) {
        let text = format!(
            "Block: {}\nPosition: {}",
            if block == 1 { "dirt" } else { "grass" },
            position.as_ivec3()
        );

        let measured = measure_text(&text, None, 32, 1.0);

        draw_multiline_text(
            &text,
            measured.width.mul_add(-0.6, screen_width()) - 12.0,
            20.0,
            32.0,
            None,
            RED,
        );

        if is_key_down(KeyCode::LeftControl) && is_key_pressed(KeyCode::F) {
            state.current_block = Some((block, position.as_ivec3()));
        }
    }
}

fn prepare(game: &mut Game) {
    let root = PathBuf::from("/home/aiving/dev/meralus/crates/app/resources/textures");

    game.load_texture("dirt", root.join("dirt.png"));
    game.load_texture("grass-block/top", root.join("grass_block_top.png"));
    game.load_texture("grass-block/side", root.join("grass_block_side.png"));
    game.load_texture(
        "grass-block/side-overlay",
        root.join("grass_block_side_overlay.png"),
    );

    game.load_block(dirt_block(game));
    game.load_block(grass_block(game));
    game.find_chunk_mut(Vec3::ZERO).unwrap().blocks[16][2][2] = 2;
}

const DEBUG_FACES: bool = true;

async fn app(mut game: Game) {
    set_default_filter_mode(FilterMode::Nearest);

    prepare(&mut game);

    let meshes = game.compute_world_mesh();

    let mut player = PlayerController {
        position: vec3(2.0, 20.0, 2.0),
        ..Default::default()
    };

    let mut last_mouse_position: Vec2 = mouse_position().into();

    let mut grabbed = true;

    set_cursor_grab(grabbed);

    show_mouse(false);

    let mut camera = Camera3D {
        position: player.position,
        up: player.up,
        target: player.position + player.front,
        ..Default::default()
    };

    // unsafe {
    //     gl::glEnable(gl::GL_CULL_FACE);
    //     gl::glCullFace(gl::GL_FRONT);
    // }

    let mut wireframe = false;
    let mut state = GameState {
        current_block: None,
    };

    loop {
        let delta = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        if is_key_pressed(KeyCode::Tab) {
            grabbed = !grabbed;

            set_cursor_grab(grabbed);

            show_mouse(!grabbed);
        }

        player.handle_physics(&game, delta);

        let mouse_position = Vec2::from(mouse_position());

        if grabbed {
            player.handle_mouse(mouse_position - last_mouse_position, delta);
        }

        last_mouse_position = mouse_position;

        clear_background(LIGHTGRAY);

        // Going 3d!

        camera.position = player.position;
        camera.up = player.up;
        camera.target = player.position + player.front;

        set_camera(&camera);

        let mut vertices = Vec::new();

        if wireframe {
            unsafe {
                gl::glPolygonMode(gl::GL_FRONT_AND_BACK, gl::GL_LINE);
            }
        }

        for backed in &meshes {
            draw_mesh(&backed.mesh);

            if DEBUG_FACES {
                debug_face_vertices(&state, backed, &mut vertices);
            }
        }

        // Back to screen space, render some text

        set_default_camera();

        if wireframe {
            unsafe {
                gl::glPolygonMode(gl::GL_FRONT_AND_BACK, gl::GL_FILL);
            }
        }

        if DEBUG_FACES {
            debug_current_block_faces(&state, &vertices);

            if state.current_block.is_some() {
                debug_current_block(&game, &mut state, &camera);
            } else {
                show_block_info(&game, &mut state, player.position - vec3(0.0, 2.0, 0.0));
            }
        }

        draw_multiline_text(
            &format!(
                "FPS: {}\nFrame time: {delta}\nYaw: {}deg\nPitch: {}\nX: {}\nY: {}\nZ: {}",
                get_fps(),
                player.yaw / LOOK_SPEED,
                player.pitch / -LOOK_SPEED,
                player.position.x,
                player.position.y,
                player.position.z
            ),
            10.0,
            20.0,
            32.0,
            None,
            BLACK,
        );

        if is_key_released(KeyCode::T) {
            wireframe = !wireframe;
        }

        next_frame().await;
    }
}
