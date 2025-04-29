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
        AsColor, CameraExt, get_movement_direction, get_rotation_directions, raycast, vertex_ao,
    },
};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use glam::{Mat4, UVec2, Vec2, Vec3, vec3};
use meralus_engine::{
    ActiveEventLoop, Application, EventLoop, KeyCode, State, Vertex, WindowDisplay,
    glium::{
        BackfaceCullingMode, Blend, Depth, DepthTest, DrawParameters, PolygonMode, Program, Rect,
        Surface, VertexBuffer,
        index::{NoIndices, PrimitiveType},
        pixel_buffer::PixelBuffer,
        uniform,
        uniforms::{MagnifySamplerFilter, MinifySamplerFilter},
        winit::{event_loop::ControlFlow, keyboard::PhysicalKey},
    },
};
use meralus_shared::{IncomingPacket, OutgoingPacket, wrap_stream};
use owo_colors::OwoColorize;
use std::{collections::HashSet, fs, net::SocketAddrV4, ops::Not};
use tokio::net::TcpStream;
use util::BufferExt;

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
    window_matrix: Mat4,
    wireframe: bool,
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

    pub fn clear(&mut self) {
        self.pressed_once.clear();
        self.released.clear();
    }
}

impl State for GameLoop {
    fn new(display: &WindowDisplay) -> Self {
        let mut game = Game::new(display, "./resources", 12723, -2..2, -2..2);

        println!(
            "[{:18}] Generated {} chunks",
            "INFO/WorldGen".bright_green(),
            game.chunks_count().bright_blue().bold(),
        );

        game.load_buitlin_blocks();

        game.generate_mipmaps(4);

        let mut draws = Vec::new();

        let world_mesh = game.compute_world_mesh();

        println!(
            "[{:18}] Generated {} meshes for chunks",
            "INFO/Rendering".bright_green(),
            (world_mesh.len() * 6).bright_blue().bold()
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
            position: vec3(2.0, 275.0, 2.0),
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
            window_matrix: Mat4::IDENTITY,
            wireframe: false,
        }
    }

    fn handle_window_resize(&mut self, _: &ActiveEventLoop, size: UVec2, scale_factor: f64) {
        let size = size.as_vec2();

        self.window_matrix = Mat4::orthographic_rh_gl(
            0.,
            size.x / scale_factor as f32,
            size.y / scale_factor as f32,
            0.,
            -1.,
            1.,
        );

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
        if self.keyboard.is_key_pressed_once(KeyCode::KeyT) {
            self.wireframe = !self.wireframe;
        }

        if self.keyboard.is_key_pressed_once(KeyCode::KeyL) {
            let atlas = self.game.get_texture_atlas();

            println!(
                "[{:18}] Saving atlas ({} packed textures) with {} mipmap levels...",
                "INFO/AtlasManager".bright_green(),
                self.game.get_texture_count().bright_blue(),
                atlas.get_mipmap_levels().bright_blue()
            );

            for level in 0..atlas.get_mipmap_levels() {
                if let Some(mipmap) = atlas.mipmap(level) {
                    let [width, height] = [mipmap.width(), mipmap.height()];
                    let buffer = PixelBuffer::new_empty(display, width as usize * height as usize);

                    if let Some(image) = mipmap.first_layer().into_image(None) {
                        image.raw_read_to_pixel_buffer(
                            &Rect {
                                left: 0,
                                bottom: 0,
                                width: mipmap.width(),
                                height: mipmap.height(),
                            },
                            &buffer,
                        );
                    }

                    let pixels = buffer.read_flatten().unwrap();

                    if let Some(image_buffer) =
                        image::ImageBuffer::from_raw(mipmap.width(), mipmap.height(), pixels)
                    {
                        let image = image::DynamicImage::ImageRgba8(image_buffer).flipv();

                        if fs::exists("debug").is_ok_and(Not::not) {
                            if let Err(error) = fs::create_dir("debug") {
                                println!(
                                    "[{:18}] Failed to create debug directory: {error}",
                                    " ERR/AtlasManager".bright_red(),
                                );

                                break;
                            }
                        }

                        if let Err(error) = image.save(format!("debug/atlas_{level}.png")) {
                            println!(
                                "[{:18}] Failed to save atlas (mipmap level: {}, size: {}): {error}",
                                " ERR/AtlasManager".bright_red(),
                                level.to_string().bright_blue(),
                                format!("{width}x{height}").bright_blue()
                            );
                        } else {
                            println!(
                                "[{:18}] Successfully saved atlas (mipmap level: {}, size: {})",
                                "INFO/AtlasManager".bright_green(),
                                level.to_string().bright_blue(),
                                format!("{width}x{height}").bright_blue()
                            );
                        }
                    }
                }
            }
        }

        let mut frame = display.draw();

        frame.clear_color_and_depth((120.0 / 255.0, 167.0 / 255.0, 1.0, 1.0), 1.0);

        for (vertex_buffer, _) in &self.draws {
            let matrix = self.camera.matrix();

            let uniforms = uniform! {
                matrix: matrix.to_cols_array_2d(),
                tex: self.game.get_texture_atlas().sampled().minify_filter(MinifySamplerFilter::NearestMipmapLinear).magnify_filter(MagnifySamplerFilter::Nearest),
                with_tex: true,
            };

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
                        backface_culling: BackfaceCullingMode::CullCounterClockwise,
                        polygon_mode: if self.wireframe {
                            PolygonMode::Line
                        } else {
                            PolygonMode::Fill
                        },
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .expect("failed to draw!");
        }

        frame.finish().expect("failed to finish draw frame");

        self.keyboard.clear();
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
