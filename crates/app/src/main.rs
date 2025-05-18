#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal,
    clippy::missing_panics_doc
)]

mod aabb;
mod blocks;
mod camera;
mod clock;
mod game;
mod keyboard;
mod loaders;
mod mesh;
mod player;
mod raycast;
mod renderers;
mod transform;
mod ui;
mod util;

use std::{f32, fs, net::SocketAddrV4, ops::Not, time::Duration};

use camera::Camera;
use clap::Parser;
use clock::Clock;
use glam::{IVec2, Mat4, UVec2, Vec2, Vec3, vec3};
use keyboard::KeyboardController;
use meralus_animation::{Animation, AnimationPlayer, Curve, RepeatMode};
use meralus_engine::{
    ActiveEventLoop, Application, EventLoop, KeyCode, State, WindowDisplay,
    glium::{
        Blend, BlendingFunction, LinearBlendingFactor, Rect, Surface,
        pixel_buffer::PixelBuffer,
        winit::{event::MouseButton, event_loop::ControlFlow},
    },
};
use meralus_shared::{Color, Lerp, Point2D, Rect2D, Size2D};
use meralus_world::{CHUNK_SIZE, Chunk, ChunkManager, SUBCHUNK_COUNT};
use owo_colors::OwoColorize;
use renderers::{FONT, FONT_BOLD, Line, ShapeRenderer, TextRenderer, VoxelRenderer};
use ui::UiContext;
use util::{BufferExt, cube_outline};

pub use self::{
    aabb::Aabb,
    game::Game,
    loaders::{BakedBlockModelLoader, Block, BlockManager, TextureLoader},
    player::PlayerController,
    transform::Transform,
    util::{AsColor, CameraExt, get_movement_direction, get_rotation_directions, vertex_ao},
};

const TEXT_COLOR: Color = Color::from_hsl(120.0, 0.5, 0.4);
const BG_COLOR: Color = Color::from_hsl(120.0, 0.4, 0.75);
const DAY_COLOR: Color = Color::from_hsl(220.0, 0.5, 0.75);
const NIGHT_COLOR: Color = Color::from_hsl(220.0, 0.35, 0.25);
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

fn get_sky_color((after_day, progress): (bool, f32)) -> Color {
    if after_day {
        DAY_COLOR.lerp(&NIGHT_COLOR, progress)
    } else {
        NIGHT_COLOR.lerp(&DAY_COLOR, progress)
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, requires = "net")]
    host: Option<SocketAddrV4>,
    #[arg(short, long, group = "net")]
    nickname: Option<String>,
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

enum Action {
    UpdateChunkMesh(IVec2),
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
    ticks: usize,
    tick_sum: usize,
    accel: Duration,
    clock: Clock,
    action_queue: Vec<Action>,
    pressed_mouse_button: Option<MouseButton>,
}

impl GameLoop {
    fn destroy_looking_at(&mut self) {
        if let Some(looking_at) = self.player.looking_at {
            let local = self.game.chunk_manager().to_chunk_local(looking_at);

            if let Some(local) = local {
                self.game.chunk_manager_mut().set_block(looking_at, 0);

                if local.y >= 255 {
                    self.game.chunk_manager_mut().set_sky_light(looking_at, 15);
                }

                self.game.update_block_sky_light(looking_at);

                let chunk = ChunkManager::to_local(looking_at);

                if local.x == 0 {
                    let chunk = chunk - IVec2::X;

                    if self.game.chunk_manager().contains_chunk(&chunk) {
                        self.action_queue.push(Action::UpdateChunkMesh(chunk));
                    }
                } else if local.x == (CHUNK_SIZE as u16 - 1) {
                    let chunk = chunk + IVec2::X;

                    if self.game.chunk_manager().contains_chunk(&chunk) {
                        self.action_queue.push(Action::UpdateChunkMesh(chunk));
                    }
                }

                if local.z == 0 {
                    let chunk = chunk - IVec2::Y;

                    if self.game.chunk_manager().contains_chunk(&chunk) {
                        self.action_queue.push(Action::UpdateChunkMesh(chunk));
                    }
                } else if local.z == (CHUNK_SIZE as u16 - 1) {
                    let chunk = chunk + IVec2::Y;

                    if self.game.chunk_manager().contains_chunk(&chunk) {
                        self.action_queue.push(Action::UpdateChunkMesh(chunk));
                    }
                }

                self.action_queue.push(Action::UpdateChunkMesh(chunk));
                self.player.update_looking_at(&self.game);
            }
        }
    }
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
            game.chunk_manager().len().bright_blue().bold(),
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
            "overlay-width",
            Animation::new(0.0, 1.0, 400, Curve::EASE_IN_OUT_EXPO, RepeatMode::Once),
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
                chunk_borders: game.chunk_manager().chunks().fold(
                    Vec::new(),
                    |mut lines, Chunk { origin, .. }| {
                        let origin = origin.as_vec2() * CHUNK_SIZE as f32;
                        let chunk_size = CHUNK_SIZE as f32;
                        let chunk_height = CHUNK_SIZE as f32 * SUBCHUNK_COUNT as f32;

                        lines.extend(cube_outline(
                            vec3(origin.x, 0.0, origin.y),
                            vec3(chunk_size, chunk_height, chunk_size),
                        ));

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
                ..Camera::default()
            },
            ticks: 0,
            tick_sum: 0,
            accel: Duration::ZERO,
            player,
            player_controllable: true,
            clock: Clock::default(),
            action_queue: Vec::new(),
            pressed_mouse_button: None,
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

    fn handle_mouse_button(&mut self, _: &ActiveEventLoop, button: MouseButton, is_pressed: bool) {
        if is_pressed {
            self.pressed_mouse_button = Some(button);
        } else if self.pressed_mouse_button == Some(button) {
            self.pressed_mouse_button.take();
        }
    }

    fn handle_mouse_motion(&mut self, _: &ActiveEventLoop, mouse_delta: Vec2) {
        if self.player_controllable {
            self.player.handle_mouse(&self.game, mouse_delta);
        }
    }

    fn tick(&mut self, _: &ActiveEventLoop, _: &WindowDisplay, _: Duration) {
        self.tick_sum += 1;

        self.clock.tick();

        let progress = self.clock.get_progress();

        self.voxel_renderer.set_sun_position(if progress > 0.5 {
            1.0 - progress
        } else {
            progress
        });

        if self
            .pressed_mouse_button
            .is_some_and(|button| button == MouseButton::Left)
        {
            self.destroy_looking_at();
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

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: Duration) {
        self.accel += delta;

        if self.accel >= Duration::from_secs(1) {
            self.ticks = self.tick_sum;
            self.accel = Duration::ZERO;
            self.tick_sum = 0;
        }

        if self.keyboard.is_key_pressed_once(KeyCode::Escape) {
            event_loop.exit();
        }

        self.animation_player.advance(delta.as_secs_f32());

        if self.keyboard.is_key_pressed_once(KeyCode::KeyR) {
            self.animation_player.enable();
            self.animation_player.play("loading-screen");
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

            if self.debugging.overlay {
                self.animation_player
                    .get_mut("overlay-width")
                    .unwrap()
                    .to(1.0);
            } else {
                self.animation_player
                    .get_mut("overlay-width")
                    .unwrap()
                    .to(0.0);
            }

            self.animation_player.play("overlay-width");
        }

        while let Some(action) = self.action_queue.pop() {
            match action {
                Action::UpdateChunkMesh(origin) => {
                    if let Some(chunk) = self.game.compute_chunk_mesh_at(&origin) {
                        self.voxel_renderer.set_chunk(display, chunk);
                    }
                }
            }
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

    #[allow(clippy::too_many_lines)]
    fn render(&mut self, _: &ActiveEventLoop, display: &WindowDisplay, delta: f32) {
        self.debugging.draw_calls = 0;
        self.debugging.vertices = 0;

        let (width, height) = display.get_framebuffer_dimensions();
        let mut frame = display.draw();

        let [r, g, b] = get_sky_color(self.clock.get_visual_progress()).to_linear();

        frame.clear_color_and_depth((r, g, b, 1.0), 1.0);

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

        if let Some(position) = self.player.looking_at {
            self.shape_renderer.set_matrix(self.camera.matrix());
            self.shape_renderer.draw_lines(
                &mut frame,
                display,
                &cube_outline(position, Vec3::ONE),
                &mut self.debugging.draw_calls,
                &mut self.debugging.vertices,
            );
            self.shape_renderer.set_default_matrix();
        }

        let animation_progress: f32 = self.animation_player.get_value("loading-screen").unwrap();

        let mut context = UiContext::new(self, display, &mut frame);

        {
            let chunk = ChunkManager::to_local(context.game_loop.player.position);

            let (hours, minutes) = {
                let time = context.game_loop.clock.time().as_secs();
                let seconds = time % 60;
                let minutes = (time - seconds) / 60 % 60;
                let hours = (time - seconds - minutes * 60) / 60 / 60;

                (hours, minutes)
            };

            let version = display.get_opengl_version();

            let text = format!(
                "OpenGL {}.{}
Free GPU memory: {}
Window size: {width}x{height}
Player position: {:.2}
Chunk: {} {}
Game Time: {hours:02}:{minutes:02}
FPS: {:.0} ({:.2}ms)
TPS: {}
Looking at {}
Draw calls: {}
Rendered vertices: {}
Animation player:",
                version.1,
                version.2,
                display
                    .get_free_video_memory()
                    .map_or_else(|| String::from("unknown"), util::format_bytes),
                context.game_loop.player.position,
                chunk.x,
                chunk.y,
                1.0 / delta,
                delta * 1000.0,
                context.game_loop.ticks,
                context
                    .game_loop
                    .player
                    .looking_at
                    .and_then(|position| context
                        .game_loop
                        .game
                        .chunk_manager()
                        .get_block(position)
                        .map(|b| if b == 1 { "dirt" } else { "grass" }))
                    .unwrap_or("nothing"),
                context.game_loop.debugging.draw_calls,
                context.game_loop.debugging.vertices,
            );

            let text_size = context.measure_text("default", &text, 18.0).unwrap();
            let overlay_width = context
                .game_loop
                .animation_player
                .get_value::<_, f32>("overlay-width")
                .unwrap();

            let text_bounds = Rect2D::new(
                Point2D::new(12.0, 12.0),
                Size2D::new((522.0 + 4.0) * overlay_width, text_size.height + 4.0),
            );

            context.bounds(text_bounds, |context, _| {
                context.fill(Color::BLACK.with_alpha(0.25));

                context.padding(2.0, |context, bounds| {
                    context.clipped(bounds, |context, bounds| {
                        context.draw_text(
                            bounds.origin.to_vector(),
                            "default",
                            text,
                            18.0,
                            Color::WHITE,
                        );
                    });
                });
            });

            for i in 0..context.game_loop.animation_player.len() {
                let (finished, elapsed, duration, text) = {
                    let (name, animation) = context.game_loop.animation_player.get_at(i).unwrap();
                    let elapsed = animation.get_elapsed();
                    let duration = animation.get_duration();

                    (
                        animation.is_finished(),
                        elapsed,
                        duration,
                        format!(
                            "#{name}: {:.2}, {:.1}% ({:.2}ms/{:.2}ms)",
                            animation.get::<f32>(),
                            (elapsed / duration) * 100.0,
                            elapsed * 1000.0,
                            duration * 1000.0
                        ),
                    )
                };

                let text_size = context.measure_text("default", &text, 18.0).unwrap();

                let offset = i as f32 * (text_size.height + 8.0);

                context.bounds(
                    Rect2D::new(
                        Point2D::new(
                            12.0,
                            text_bounds.origin.y + 2.0 + text_bounds.size.height + offset,
                        ),
                        Size2D::new((522.0 + 4.0) * overlay_width, text_size.height + 6.0),
                    ),
                    |context, root| {
                        context.fill(Color::BLACK.with_alpha(0.25));

                        context.padding(2.0, |context, bounds| {
                            context.clipped(bounds, |context, bounds| {
                                context.draw_text(
                                    bounds.origin.to_vector(),
                                    "default",
                                    text,
                                    18.0,
                                    Color::WHITE,
                                );

                                context.draw_rect(
                                    (root.origin + Point2D::new(0.0, text_size.height + 4.0))
                                        .to_vector(),
                                    Size2D::new(root.size.width * (elapsed / duration), 2.0),
                                    if finished {
                                        Color::new(120, 255, 155, 255)
                                    } else {
                                        Color::new(120, 167, 255, 255)
                                    },
                                );
                            });
                        });
                    },
                );
            }
        }

        context.ui(|context, bounds| {
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

    //     if let Some(Ok(OutgoingPacket::PlayersList { players })) =
    // stream.next().await {         println!("{players:#?}");
    //     }
    // }

    let mut app = Application::<GameLoop>::default();

    let event_loop = EventLoop::builder().build().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
