#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal,
    clippy::missing_panics_doc
)]

mod block;
mod game;
mod loaders;
mod mesh;
mod player;
mod transform;
mod ui;
mod util;

mod shader {
    pub const VERTEX: &str = include_str!("../resources/shaders/common_vertex.glsl");
    pub const FRAGMENT: &str = include_str!("../resources/shaders/common_fragment.glsl");
}

pub use self::{
    block::Block,
    game::{BackedFace, Game, GameState},
    loaders::{BlockLoader, BlockModel, BlockModelFace, BlockModelLoader, TextureLoader},
    player::PlayerController,
    transform::Transform,
    util::{
        CameraExt, Vec3Ext, get_movement_direction, get_rotation_directions,
        get_vertice_neighbours, raycast, vertex_ao,
    },
};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use glam::{Mat4, UVec2, Vec2, Vec3, vec3};
use meralus_engine::{
    ActiveEventLoop, Application, EventLoop, KeyCode, State, Vertex, WindowDisplay,
    glium::{
        Depth, DepthTest, DrawParameters, Program, Surface, VertexBuffer,
        index::{NoIndices, PrimitiveType},
        uniform,
        uniforms::MagnifySamplerFilter,
        winit::{event_loop::ControlFlow, keyboard::PhysicalKey},
    },
};
use meralus_shared::{IncomingPacket, OutgoingPacket, wrap_stream};
use owo_colors::OwoColorize;
use std::{collections::HashSet, net::SocketAddrV4};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, requires = "net")]
    host: Option<SocketAddrV4>,
    #[arg(short, long, group = "net")]
    nickname: Option<String>,
}

#[derive(Debug)]
pub struct Camera3D {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fovy: f32,
    pub aspect: f32,

    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            position: vec3(0., -10., 0.),
            target: vec3(0., 0., 0.),
            aspect: 1024.0 / 768.0,
            up: vec3(0., 0., 1.),
            fovy: 45.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 10000.0,
        }
    }
}

impl Camera3D {
    fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fovy, self.aspect, self.z_near, self.z_far)
            * Mat4::look_at_rh(self.position, self.target, self.up)
    }
}

#[derive(Debug, Default)]
pub struct KeyboardController {
    pressed: HashSet<KeyCode>,
    pressed_once: HashSet<KeyCode>,
    released: HashSet<KeyCode>,
}

struct GameLoop {
    game: Game,
    keyboard: KeyboardController,
    program: Program,
    camera: Camera3D,
    player: PlayerController,
    draws: Vec<(VertexBuffer<Vertex>, usize)>,
}

impl KeyboardController {
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn is_key_pressed_once(&self, key: KeyCode) -> bool {
        self.pressed_once.contains(&key)
    }

    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.released.contains(&key)
    }
}

impl State for GameLoop {
    fn new(display: &WindowDisplay) -> Self {
        let mut game = Game::new("./resources", 12723, -4..4, -4..4);

        println!(
            "[{:18}] Generated {} chunks",
            "INFO/WorldGen".bright_green(),
            game.chunks_count().bright_blue().bold(),
        );

        game.load_buitlin_blocks(display);

        let mut draws = Vec::new();

        let world_mesh = game.compute_world_mesh();

        println!(
            "[{:18}] Generated {} meshes for chunks",
            "INFO/Rendering".bright_green(),
            world_mesh.len().bright_blue().bold()
        );

        for meshes in world_mesh {
            for mesh in meshes {
                draws.push((
                    VertexBuffer::new(display, &mesh.vertices).unwrap(),
                    mesh.texture_id,
                ));
            }
        }

        println!(
            "[{:18}] All DrawCall's for OpenGL created",
            "INFO/Rendering".bright_green(),
        );

        let player = PlayerController {
            position: vec3(2.0, 200.0, 2.0),
            ..Default::default()
        };

        Self {
            game,
            draws,
            camera: Camera3D {
                position: player.position,
                up: player.up,
                target: player.position + player.front,
                ..Default::default()
            },
            player,
            program: Program::from_source(display, shader::VERTEX, shader::FRAGMENT, None).unwrap(),
            keyboard: KeyboardController::default(),
        }
    }

    fn handle_window_resize(&mut self, _: &ActiveEventLoop, size: UVec2) {
        let size = size.as_vec2();

        self.camera.aspect = size.x / size.y;
    }

    fn handle_keyboard_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: meralus_engine::glium::winit::event::KeyEvent,
    ) {
        if let PhysicalKey::Code(code) = event.physical_key {
            if event.state.is_pressed() {
                self.keyboard.pressed_once.insert(code);

                if !event.repeat && self.keyboard.pressed.contains(&code) {
                    self.keyboard.pressed.remove(&code);
                }

                self.keyboard.pressed.insert(code);
            } else {
                if code == KeyCode::Escape {
                    event_loop.exit();
                }

                self.keyboard.pressed.remove(&code);
                self.keyboard.released.insert(code);
            }
        }
    }

    fn handle_mouse_motion(&mut self, _: &ActiveEventLoop, mouse_delta: Vec2) {
        self.player.handle_mouse(&mut None, &self.game, mouse_delta);
    }

    fn fixed_update(&mut self, _: &ActiveEventLoop, _: &WindowDisplay, delta: f32) {
        self.player
            .handle_physics(&self.game, &self.keyboard, &mut self.camera, delta);

        self.camera.position = self.player.position;
        self.camera.up = self.player.up;
        self.camera.target = self.player.position + self.player.front;
    }

    fn render(&mut self, _: &ActiveEventLoop, display: &WindowDisplay) {
        // println!("DRAWING!");

        let mut frame = display.draw();

        frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        // println!("draws: {}", self.draws.len());

        for (vertex_buffer, /* index_buffer, */ texture_id) in &self.draws {
            let matrix = self.camera.matrix();

            let uniforms = uniform! {
                matrix: matrix.to_cols_array_2d(),
                tex: self.game.get_texture_by_id(*texture_id).unwrap().sampled().magnify_filter(MagnifySamplerFilter::Nearest),
            };

            // println!("trying to render");
            frame
                .draw(
                    vertex_buffer,
                    NoIndices(PrimitiveType::TrianglesList),
                    &self.program,
                    &uniforms,
                    &DrawParameters {
                        depth: Depth {
                            test: DepthTest::IfLessOrEqual,
                            write: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )
                .expect("failed to draw!");
        }

        frame.finish().expect("failed to finish draw frame");
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Some(host) = args.host {
        let stream = TcpStream::connect(host).await.unwrap();
        let (mut stream, mut sink) = wrap_stream(stream);

        sink.send(IncomingPacket::PlayerConnected {
            name: args.nickname.unwrap(),
        })
        .await
        .unwrap();

        sink.send(IncomingPacket::GetPlayers).await.unwrap();

        if let Some(Ok(OutgoingPacket::PlayersList { players })) = stream.next().await {
            println!("{players:#?}");
        }
    }

    let mut app = Application::<GameLoop>::default();
    let event_loop = EventLoop::builder().build().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
