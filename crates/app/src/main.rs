#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal,
    clippy::missing_panics_doc
)]

mod blocks;
mod game;
mod loaders;
mod mesh;
mod player;
mod renderers;
mod transform;
mod ui;
mod util;

pub use self::{
    game::{BackedFace, Game, GameState},
    loaders::{
        BakedBlockModel, BakedBlockModelLoader, Block, BlockManager, BlockModelFace, TextureLoader,
    },
    player::PlayerController,
    transform::Transform,
    util::{
        AsColor, CameraExt, get_movement_direction, get_rotation_directions, raycast, vertex_ao,
    },
};
use clap::Parser;
use glam::{IVec2, Mat4, UVec2, Vec2, Vec3, vec3};
use meralus_animation::{Animation, AnimationPlayer, Curve, RepeatMode, RestartBehaviour};
use meralus_engine::{
    ActiveEventLoop, Application, EventLoop, KeyCode, State, WindowDisplay,
    glium::{
        Blend, BlendingFunction, LinearBlendingFactor, Rect, Surface,
        pixel_buffer::PixelBuffer,
        winit::{event::KeyEvent, event_loop::ControlFlow, keyboard::PhysicalKey},
    },
};
use meralus_shared::{Color, FromValue, Point2D, Rect2D, Size2D};
use meralus_world::{CHUNK_SIZE, Chunk, SUBCHUNK_COUNT};
use owo_colors::OwoColorize;
use renderers::{FONT, FONT_BOLD, Line, ShapeRenderer, TextRenderer, VoxelRenderer};
use std::fmt::Write;
use std::{collections::HashSet, fs, net::SocketAddrV4, ops::Not};
use ui::UiContext;
use util::BufferExt;

const TEXT_COLOR: Color = Color::from_hsl(135.0, 0.15, 0.25);
const BG_COLOR: Color = Color::new(126, 230, 152, 255);
const BLENDING: Blend = Blend {
    color: BlendingFunction::Addition {
        source: LinearBlendingFactor::SourceAlpha,
        destination: LinearBlendingFactor::OneMinusSourceAlpha,
    },
    alpha: BlendingFunction::Addition {
        source: LinearBlendingFactor::One,
        destination: LinearBlendingFactor::OneMinusSourceAlpha,
    },
    constant_value: (0.0, 0.0, 0.0, 0.0),
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, requires = "net")]
    host: Option<SocketAddrV4>,
    #[arg(short, long, group = "net")]
    nickname: Option<String>,
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: vec3(0., -10., 0.),
            target: vec3(0., 0., 0.),
            aspect_ratio: 1024.0 / 768.0,
            up: vec3(0., 0., 1.),
            fov: 55.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 10000.0,
        }
    }
}

impl Camera {
    fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov, self.aspect_ratio, self.z_near, self.z_far)
            * Mat4::look_at_rh(self.position, self.target, self.up)
    }
}

#[derive(Debug, Default)]
pub struct KeyboardController {
    pressed: HashSet<KeyCode>,
    pressed_once: HashSet<KeyCode>,
    released: HashSet<KeyCode>,
}

#[allow(clippy::struct_excessive_bools)]
struct Debugging {
    night: bool,
    overlay: bool,
    wireframe: bool,
    draw_borders: bool,
    chunk_borders: Vec<Line>,
    vertices: usize,
    draw_calls: usize,
}

struct GameLoop {
    game: Game,
    keyboard: KeyboardController,
    camera: Camera,
    player: PlayerController,
    window_matrix: Mat4,
    debugging: Debugging,
    player_controllable: bool,
    animation_player: AnimationPlayer,
    text_renderer: TextRenderer,
    voxel_renderer: VoxelRenderer,
    shape_renderer: ShapeRenderer,
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

    pub fn handle_keyboard_input(&mut self, event: &KeyEvent) {
        if let PhysicalKey::Code(code) = event.physical_key {
            if event.state.is_pressed() {
                if !event.repeat {
                    self.pressed_once.insert(code);

                    if self.pressed.contains(&code) {
                        self.pressed.remove(&code);
                    }
                }

                self.pressed.insert(code);
            } else {
                self.pressed.remove(&code);
                self.released.insert(code);
            }
        }
    }
}

fn chunk_borders(origin: IVec2) -> [Line; 12] {
    let origin = origin.as_vec2() * CHUNK_SIZE as f32;
    let chunk_size = CHUNK_SIZE as f32;
    let chunk_height = CHUNK_SIZE as f32 * SUBCHUNK_COUNT as f32;

    [
        [[0.0, 0.0, 0.0], [0.0, chunk_height, 0.0]],
        [[chunk_size, 0.0, 0.0], [chunk_size, chunk_height, 0.0]],
        [[0.0, 0.0, chunk_size], [0.0, chunk_height, chunk_size]],
        [
            [chunk_size, 0.0, chunk_size],
            [chunk_size, chunk_height, chunk_size],
        ],
        [[0.0, 0.0, 0.0], [chunk_size, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, chunk_size]],
        [[chunk_size, 0.0, 0.0], [chunk_size, 0.0, chunk_size]],
        [[0.0, 0.0, chunk_size], [chunk_size, 0.0, chunk_size]],
        [[0.0, chunk_height, 0.0], [chunk_size, chunk_height, 0.0]],
        [[0.0, chunk_height, 0.0], [0.0, chunk_height, chunk_size]],
        [
            [chunk_size, chunk_height, 0.0],
            [chunk_size, chunk_height, chunk_size],
        ],
        [
            [0.0, chunk_height, chunk_size],
            [chunk_size, chunk_height, chunk_size],
        ],
    ]
    .map(|[start, end]| {
        Line::new(
            Vec3::new(origin.x, 0.0, origin.y) + Vec3::from_array(start),
            Vec3::new(origin.x, 0.0, origin.y) + Vec3::from_array(end),
            Color::BLUE,
        )
    })
}

const DAY_COLOR: Color = Color::from_hsl(220.0, 1.0, 0.75);
const NIGHT_COLOR: Color = Color::new(5, 10, 20, 255);

const fn get_sky_color(night: bool) -> &'static Color {
    if night { &NIGHT_COLOR } else { &DAY_COLOR }
}

impl State for GameLoop {
    fn new(display: &WindowDisplay) -> Self {
        let mut game = Game::new(display, "./resources", -2..2, -2..2);

        game.load_buitlin_blocks();
        game.generate_mipmaps(4);

        game.generate_world(12723);
        game.generate_lights();
        game.set_block_light(vec3(-13.0, 217.0, 0.0), 15);

        println!(
            "[{:18}] Generated {} chunks",
            "INFO/WorldGen".bright_green(),
            game.chunk_manager.len().bright_blue().bold(),
        );

        let world_mesh = game.compute_world_mesh();

        println!(
            "[{:18}] Generated {} meshes for chunks",
            "INFO/Rendering".bright_green(),
            (world_mesh.len() * 6).bright_blue().bold()
        );

        let player = PlayerController {
            position: vec3(2.0, 275.0, 2.0),
            ..Default::default()
        };

        let mut text_renderer = TextRenderer::new(display, 4096).unwrap();

        text_renderer.add_font(display, "default", FONT);
        text_renderer.add_font(display, "default_bold", FONT_BOLD);

        let mut animation_player = AnimationPlayer::default();

        animation_player.add(
            "loading-screen",
            Animation::new(1.0, 0.0, 1000, Curve::LINEAR, RepeatMode::Once),
        );

        animation_player.add(
            "xd",
            Animation::new(
                0.0,
                192.0,
                400,
                Curve::EASE_IN_OUT_EXPO,
                RepeatMode::Infinite,
            )
            .with_restart_behaviour(RestartBehaviour::EndValue),
        );

        let mut voxel_renderer = VoxelRenderer::new(display, world_mesh);

        voxel_renderer.set_sun_position(0.5);

        Self {
            keyboard: KeyboardController::default(),
            animation_player,
            text_renderer,
            voxel_renderer,
            shape_renderer: ShapeRenderer::new(display),
            window_matrix: Mat4::IDENTITY,
            debugging: Debugging {
                night: false,
                overlay: false,
                wireframe: false,
                draw_borders: false,
                chunk_borders: game.chunk_manager.chunks().fold(
                    Vec::new(),
                    |mut lines, Chunk { origin, .. }| {
                        lines.extend(chunk_borders(*origin));

                        lines
                    },
                ),
                vertices: 0,
                draw_calls: 0,
            },
            game,
            camera: Camera {
                position: player.position,
                up: player.up,
                target: player.position + player.front,
                ..Default::default()
            },
            player,
            player_controllable: true,
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

        self.camera.aspect_ratio = size.x / size.y;
    }

    fn handle_keyboard_input(
        &mut self,
        _: &ActiveEventLoop,
        event: meralus_engine::glium::winit::event::KeyEvent,
    ) {
        self.keyboard.handle_keyboard_input(&event);
    }

    fn handle_mouse_motion(&mut self, _: &ActiveEventLoop, mouse_delta: Vec2) {
        if self.player_controllable {
            self.player.handle_mouse(&mut None, &self.game, mouse_delta);
        }
    }

    fn fixed_update(&mut self, _: &ActiveEventLoop, _: &WindowDisplay, delta: f32) {
        if self.player_controllable {
            self.player
                .handle_physics(&self.game, &self.keyboard, &mut self.camera, delta);

            self.camera.position = self.player.position;
            self.camera.up = self.player.up;
            self.camera.target = self.player.position + self.player.front;
        }
    }

    fn update(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: f32) {
        if self.keyboard.is_key_pressed_once(KeyCode::Escape) {
            event_loop.exit();
        }

        self.animation_player.advance(delta);

        if self.keyboard.is_key_pressed_once(KeyCode::KeyR) {
            self.animation_player.reset();
            self.animation_player.enable();
        }

        if self.keyboard.is_key_pressed_once(KeyCode::KeyT) {
            self.debugging.wireframe = !self.debugging.wireframe;
        }

        if self.keyboard.is_key_pressed_once(KeyCode::KeyN) {
            self.debugging.night = !self.debugging.night;

            self.voxel_renderer
                .set_sun_position(if self.debugging.night { -0.5 } else { 0.5 });
        }

        if self.keyboard.is_key_pressed_once(KeyCode::KeyO) {
            self.debugging.overlay = !self.debugging.overlay;
        }

        if self.keyboard.is_key_pressed_once(KeyCode::KeyB) {
            self.debugging.draw_borders = !self.debugging.draw_borders;
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
    }

    fn render(&mut self, _: &ActiveEventLoop, display: &WindowDisplay, delta: f32) {
        self.debugging.draw_calls = 0;
        self.debugging.vertices = 0;

        let (width, height) = display.get_framebuffer_dimensions();
        let mut frame = display.draw();

        frame.clear_color_and_depth(
            <[f32; 4]>::from_value(get_sky_color(self.debugging.night)).into(),
            1.0,
        );

        self.voxel_renderer.render(
            &mut frame,
            self.camera.matrix(),
            self.game.get_texture_atlas_sampled(),
            self.debugging.wireframe,
        );

        let (draw_calls, vertices) = self.voxel_renderer.get_debug_info();

        self.debugging.draw_calls += draw_calls;
        self.debugging.vertices += vertices;

        if self.debugging.draw_borders {
            self.shape_renderer.set_matrix(self.camera.matrix());
            self.shape_renderer.draw_lines(
                &mut frame,
                display,
                &self.debugging.chunk_borders,
                &mut self.debugging.draw_calls,
                &mut self.debugging.vertices,
            );
            self.shape_renderer.set_default_matrix();
        }

        let animation_progress: f32 = self.animation_player.get_value("loading-screen").unwrap();
        let xd_progress: f32 = self.animation_player.get_value("xd").unwrap();

        let mut context = UiContext::new(self, display, &mut frame);

        context.ui(|context, bounds| {
            context.draw_rect(
                Point2D::new(256.0 + xd_progress, 12.0),
                Size2D::new(48.0, 48.0),
                Color::new(120, 167, 255, 255),
            );

            context.fill(BG_COLOR.with_alpha(animation_progress));

            let measured = context
                .measure_text("default_bold", "Meralus", 64.0)
                .unwrap();
            let text_pos = Point2D::from_size((bounds.size - measured) / 2.0);

            let progress_width = bounds.size.width * 0.5;
            let progress_position = (bounds.size.width - progress_width) / 2.0;
            let offset = Point2D::new(progress_position, text_pos.y + 12.0 + measured.height);

            context.bounds(
                Rect2D::new(bounds.origin + offset, Size2D::new(progress_width, 48.0)),
                |context, _| {
                    context.fill(TEXT_COLOR.with_alpha(animation_progress));

                    context.padding(2.0, |context, _| {
                        context.fill(BG_COLOR.with_alpha(animation_progress));

                        context.padding(2.0, |context, bounds| {
                            context.draw_rect(
                                bounds.origin.into(),
                                bounds
                                    .size
                                    .with_width(bounds.size.width * (1.0 - animation_progress)),
                                TEXT_COLOR.with_alpha(animation_progress),
                            );
                        });
                    });
                },
            );

            context.draw_text(
                text_pos,
                "default_bold",
                "Meralus",
                64.0,
                TEXT_COLOR.with_alpha(animation_progress),
            );
        });

        context.finish();

        if self.debugging.overlay {
            self.text_renderer.render(
                &mut frame,
                &self.window_matrix,
                Point2D::new(12.0, 12.0),
                "default",
                format!(
                    "Free GPU memory: {}\nWindow size: {width}x{height}\nPlayer position: {:.2}\nFPS: {:.0} ({:.2}ms)\nDraw calls: {}\nRendered vertices: {}\nAnimation player:{}",
                    display.get_free_video_memory().map_or_else(|| String::from("unknown"), util::format_bytes),
                    self.player.position,
                    1.0 / delta,
                    delta * 1000.0,
                    self.debugging.draw_calls,
                    self.debugging.vertices,
                    self.animation_player.animations().fold(String::new(), |mut data, (name, animation)| {
                        let elapsed = animation.get_elapsed();
                        let duration = animation.get_duration();

                        write!(data, "\n |\n ---- #{name}: {:.1}% ({:.2}ms/{:.2}ms)", (elapsed / duration) * 100.0, elapsed * 1000.0, duration * 1000.0).unwrap();

                        data
                    })
                ),
                18.0,
                TEXT_COLOR,
                &mut self.debugging.draw_calls
            );
        }

        frame.finish().expect("failed to finish draw frame");

        self.keyboard.clear();
    }
}

#[tokio::main]
async fn main() {
    // let args = Args::parse();

    // if let Some(host) = args.host {
    //     let stream = TcpStream::connect(host).await.unwrap();
    //     let (mut stream, mut sink) = wrap_stream(stream);

    //     sink.send(IncomingPacket::PlayerConnected {
    //         name: args.nickname.unwrap(),
    //     })
    //     .await
    //     .unwrap();

    //     sink.send(IncomingPacket::GetPlayers).await.unwrap();

    //     if let Some(Ok(OutgoingPacket::PlayersList { players })) = stream.next().await {
    //         println!("{players:#?}");
    //     }
    // }

    let mut app = Application::<GameLoop>::default();

    let event_loop = EventLoop::builder().build().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
