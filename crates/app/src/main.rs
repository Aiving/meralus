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
mod player;
mod raycast;
mod renderers;
mod transform;
mod ui;
mod util;

use std::{f32, fs, net::SocketAddrV4, ops::Not, time::Duration};

use blocks::{AirBlock, DirtBlock, GrassBlock};
use camera::Camera;
use clap::Parser;
use clock::Clock;
use glam::{IVec2, Mat4, Quat, UVec2, Vec2, Vec3, vec3};
use glamour::{FromRaw, ToRaw};
use glium::{
    Blend, BlendingFunction, LinearBlendingFactor, Rect, Surface, pixel_buffer::PixelBuffer,
};
use keyboard::KeyboardController;
use meralus_animation::{Animation, AnimationPlayer, Curve, RepeatMode};
use meralus_engine::{
    Application, CursorGrabMode, KeyCode, MouseButton, State, WindowContext, WindowDisplay,
};
use meralus_shared::{Color, Cube3D, Lerp, Point2D, Point3D, Rect2D, Size2D, Size3D};
use meralus_world::{CHUNK_HEIGHT_F32, CHUNK_SIZE_F32, CHUNK_SIZE_U16, Chunk, ChunkManager};
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

pub const TICK_RATE: Duration = Duration::from_millis(50);
pub const FIXED_FRAMERATE: Duration = Duration::from_secs(1)
    .checked_div(60)
    .expect("failed to calculate fixed framerate somehow");
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
    inventory_open: bool,
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
    fixed_accel: Duration,
    tick_accel: Duration,

    inventory_slot: u8,
}

const INVENTORY_HOTBAR_SLOTS: u8 = 10;

impl GameLoop {
    fn destroy_looking_at(&mut self) {
        if let Some(looking_at) = self.player.looking_at {
            let local = self
                .game
                .chunk_manager()
                .to_chunk_local(looking_at.position);

            if let Some(local) = local {
                self.game
                    .chunk_manager_mut()
                    .set_block(looking_at.position, 0);

                if local.y >= 255 {
                    self.game
                        .chunk_manager_mut()
                        .set_sky_light(looking_at.position, 15);
                }

                self.game.update_block_sky_light(looking_at.position);

                let chunk = ChunkManager::to_local(looking_at.position);

                if local.x == 0 {
                    let chunk = chunk - IVec2::X;

                    if self.game.chunk_manager().contains_chunk(&chunk) {
                        self.action_queue.push(Action::UpdateChunkMesh(chunk));
                    }
                } else if local.x == (CHUNK_SIZE_U16 - 1) {
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
                } else if local.z == (CHUNK_SIZE_U16 - 1) {
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

    fn tick(&mut self) {
        self.tick_sum += 1;

        self.clock.tick();

        let progress = self.clock.get_progress();

        self.voxel_renderer.set_sun_position(if progress > 0.5 {
            1.0 - progress
        } else {
            progress
        });
    }

    fn fixed_update(&mut self) {
        if self.player_controllable {
            self.player.handle_physics(
                &self.game,
                &self.keyboard,
                &mut self.camera,
                FIXED_FRAMERATE.as_secs_f32(),
            );

            self.camera.position = self.player.position;
            self.camera.up = self.player.up;
            self.camera.target = self.player.position + self.player.front;

            self.player.frustum.update(self.camera.matrix());
        }
    }
}

const SLOT_SIZE: f32 = 48.0f32;

impl State for GameLoop {
    fn new(context: WindowContext, display: &WindowDisplay) -> Self {
        context.set_cursor_grab(CursorGrabMode::Confined);
        context.set_cursor_visible(false);

        let mut game = Game::new(display, "./resources", -3..3, -3..3);

        game.register_block(AirBlock);
        game.register_block(DirtBlock);
        game.register_block(GrassBlock);

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

        let mut text_renderer = TextRenderer::new(display, 4096 / 2).unwrap();

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

        animation_player.add(
            "scale",
            Animation::new(0.0, 1.0, 400, Curve::EASE_IN_OUT_EXPO, RepeatMode::Once),
        );

        animation_player.add(
            "opacity",
            Animation::new(0.0, 1.0, 400, Curve::LINEAR, RepeatMode::Once),
        );

        animation_player.add(
            "scale-vertical",
            Animation::new(0.0, 1.0, 400, Curve::EASE_IN_OUT_EXPO, RepeatMode::Once),
        );

        Self {
            keyboard: KeyboardController::default(),
            animation_player,
            text_renderer,
            voxel_renderer: VoxelRenderer::new(display, world_mesh),
            shape_renderer: ShapeRenderer::new(display),
            window_matrix: Mat4::IDENTITY,
            debugging: Debugging {
                night: false,
                overlay: false,
                wireframe: false,
                draw_borders: false,
                inventory_open: false,
                chunk_borders: game.chunk_manager().chunks().fold(
                    Vec::new(),
                    |mut lines, Chunk { origin, .. }| {
                        let origin = origin.as_vec2() * CHUNK_SIZE_F32;

                        lines.extend(cube_outline(Cube3D::new(
                            Point3D::new(origin.x, 0.0, origin.y),
                            Size3D::new(CHUNK_SIZE_F32, CHUNK_HEIGHT_F32, CHUNK_SIZE_F32),
                        )));

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
            fixed_accel: Duration::ZERO,
            tick_accel: Duration::ZERO,
            player,
            player_controllable: true,
            clock: Clock::default(),
            action_queue: Vec::new(),
            inventory_slot: 0,
        }
    }

    fn handle_window_resize(&mut self, size: UVec2, scale_factor: f64) {
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

    fn handle_keyboard_input(&mut self, key: KeyCode, is_pressed: bool, repeat: bool) {
        self.keyboard.handle_keyboard_input(key, is_pressed, repeat);
    }

    fn handle_mouse_button(&mut self, button: MouseButton, is_pressed: bool) {
        if button == MouseButton::Left && is_pressed {
            self.destroy_looking_at();
        }
    }

    fn handle_mouse_motion(&mut self, mouse_delta: Vec2) {
        if self.player_controllable {
            self.player.handle_mouse(&self.game, mouse_delta);
        }
    }

    fn handle_mouse_wheel(&mut self, delta: Vec2) {
        if delta.y > 0.0 {
            if self.inventory_slot == INVENTORY_HOTBAR_SLOTS - 1 {
                self.inventory_slot = 0;
            } else {
                self.inventory_slot += 1;
            }
        } else if delta.y < 0.0 {
            if self.inventory_slot == 0 {
                self.inventory_slot = INVENTORY_HOTBAR_SLOTS - 1;
            } else {
                self.inventory_slot -= 1;
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, context: WindowContext, display: &WindowDisplay, delta: Duration) {
        self.accel += delta;
        self.fixed_accel += delta;
        self.tick_accel += delta;

        while self.fixed_accel > FIXED_FRAMERATE {
            self.fixed_accel -= FIXED_FRAMERATE;

            self.fixed_update();
        }

        while self.tick_accel > TICK_RATE {
            self.tick_accel -= TICK_RATE;

            self.tick();
        }

        if self.accel >= Duration::from_secs(1) {
            self.ticks = self.tick_sum;
            self.accel = Duration::ZERO;
            self.tick_sum = 0;
        }

        if self.keyboard.is_key_pressed_once(KeyCode::Tab) {
            self.player_controllable = !self.player_controllable;

            if self.player_controllable {
                context.set_cursor_grab(CursorGrabMode::Confined);
                context.set_cursor_visible(false);
            } else {
                context.set_cursor_grab(CursorGrabMode::None);
                context.set_cursor_visible(true);
            }
        }

        if self.keyboard.is_key_pressed_once(KeyCode::Escape) {
            context.close_window();
        }

        self.animation_player.advance(delta.as_secs_f32());

        if self.keyboard.is_key_pressed_once(KeyCode::KeyR) {
            self.animation_player.enable();
            self.animation_player.play("loading-screen");
        }

        if self.keyboard.is_key_pressed_once(KeyCode::KeyT) {
            self.debugging.wireframe = !self.debugging.wireframe;
        }

        if self.keyboard.is_key_pressed_once(KeyCode::KeyV) {
            self.debugging.inventory_open = !self.debugging.inventory_open;

            if self.debugging.inventory_open {
                let scale = self.animation_player.get_mut("scale").unwrap();

                scale.set_delay(0);
                scale.to(1.0);

                let scale_vertical = self.animation_player.get_mut("scale-vertical").unwrap();

                scale_vertical.set_delay(400);
                scale_vertical.to(1.0);

                let opacity = self.animation_player.get_mut("opacity").unwrap();

                opacity.set_delay(0);
                opacity.to(1.0);
            } else {
                let scale = self.animation_player.get_mut("scale").unwrap();

                scale.set_delay(400);
                scale.to(0.0);

                let scale_vertical = self.animation_player.get_mut("scale-vertical").unwrap();

                scale_vertical.set_delay(0);
                scale_vertical.to(0.0);

                let opacity = self.animation_player.get_mut("opacity").unwrap();

                opacity.set_delay(400);
                opacity.to(0.0);
            }

            self.animation_player.play("scale");
            self.animation_player.play("opacity");
            self.animation_player.play("scale-vertical");
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
                        self.voxel_renderer.set_chunk(display, origin, chunk);
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

                        if fs::exists("debug").is_ok_and(Not::not)
                            && let Err(error) = fs::create_dir("debug")
                        {
                            println!(
                                "[{:18}] Failed to create debug directory: {error}",
                                " ERR/AtlasManager".bright_red(),
                            );

                            break;
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
    fn render(&mut self, display: &WindowDisplay, delta: Duration) {
        let draw_calls = self.debugging.draw_calls;
        let vertices = self.debugging.vertices;

        self.debugging.draw_calls = 0;
        self.debugging.vertices = 0;

        let (width, height) = display.get_framebuffer_dimensions();
        let mut frame = display.draw();

        let [r, g, b] = get_sky_color(self.clock.get_visual_progress()).to_linear();

        frame.clear_color_and_depth((r, g, b, 1.0), 1.0);

        self.voxel_renderer.render(
            &mut frame,
            &self.player.frustum,
            self.camera.matrix(),
            self.game.get_texture_atlas_sampled(),
            self.debugging.wireframe,
        );

        {
            let (draw_calls, vertices) = self.voxel_renderer.get_debug_info();

            self.debugging.draw_calls += draw_calls;
            self.debugging.vertices += vertices;
        }

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

        if let Some(result) = self.player.looking_at
            && let Some(model) = self.game.get_model_for(result.position)
        {
            self.shape_renderer.set_matrix(self.camera.matrix());
            self.shape_renderer.draw_lines(
                &mut frame,
                display,
                &cube_outline(model.bounding_box + Point3D::from_raw(result.position)),
                &mut self.debugging.draw_calls,
                &mut self.debugging.vertices,
            );
            self.shape_renderer.set_default_matrix();
        }

        let animation_progress: f32 = self.animation_player.get_value("loading-screen").unwrap();

        let mut context = UiContext::new(self, display, &mut frame);

        context.ui(|context, bounds| {
            let hotbar_width = f32::from(INVENTORY_HOTBAR_SLOTS) * SLOT_SIZE;

            let origin = Point2D::new(
                (bounds.size.width / 2.0) - (hotbar_width / 2.0),
                bounds.size.height - SLOT_SIZE - 8.0,
            );

            let offset = f32::from(context.game_loop.inventory_slot) * SLOT_SIZE;

            context.draw_rect(
                origin,
                Size2D::new(hotbar_width, SLOT_SIZE),
                Color::from_hsl(0.0, 0.0, 0.5),
            );

            context.draw_rect(
                origin + Point2D::new(offset, 0.0).into(),
                Size2D::new(SLOT_SIZE, SLOT_SIZE),
                Color::from_hsl(0.0, 0.0, 0.8),
            );

            context.draw_rect(
                origin + Point2D::new(4.0, 4.0).into() + Point2D::new(offset, 0.0).into(),
                Size2D::new(SLOT_SIZE - 8.0, SLOT_SIZE - 8.0),
                Color::from_hsl(0.0, 0.0, 0.5),
            );
        });

        context.ui(|context, bounds| {
            let opacity: f32 = context
                .game_loop
                .animation_player
                .get_value("opacity")
                .unwrap();

            let scale: f32 = context
                .game_loop
                .animation_player
                .get_value("scale")
                .unwrap();

            let scale_vertical: f32 = context
                .game_loop
                .animation_player
                .get_value("scale-vertical")
                .unwrap();

            let screen_center = bounds.center();

            let size = Size2D::new(bounds.size.width * 0.65, bounds.size.height * 0.4)
                + (Size2D::new(0.0, 320.0) * scale_vertical);
            let center = screen_center - (size / 2.0).to_vector();

            context.add_transform(Mat4::from_scale_rotation_translation(
                Vec3::from_array([scale; 3]),
                Quat::IDENTITY,
                screen_center.to_raw().extend(0.0) * (1.0 - scale),
            ));

            context.bounds(Rect2D::new(center, size), |context, _| {
                context.fill(Color::from_hsl(130.0, 0.35, 0.25).with_alpha(opacity));

                context.padding(2.0, |context, bounds| {
                    context.clipped(bounds, |context, bounds| {
                        let measured = context
                            .measure_text("default_bold", "Inventory", 18.0)
                            .unwrap();

                        context.draw_text(
                            bounds.origin,
                            "default_bold",
                            "Inventory",
                            18.0,
                            Color::WHITE,
                        );

                        let size = bounds.size - Size2D::new(0.0, measured.height + 4.0);
                        let origin =
                            bounds.origin + Size2D::new(0.0, measured.height + 2.0).to_vector();

                        let inner_origin = origin + Point2D::new(2.0, 2.0).to_vector();
                        let inner_size = size - Size2D::new(4.0, 4.0);

                        let tile_count = 24usize;
                        let tile_gap = 2.0f32;
                        let tile_size = (inner_size
                            - Size2D::new(
                                (tile_count as f32 - 1.0) * tile_gap,
                                (tile_count as f32 - 1.0) * tile_gap,
                            ))
                            / tile_count as f32;

                        context.draw_rect(
                            origin,
                            size,
                            Color::from_hsl(130.0, 0.5, 0.75).with_alpha(opacity),
                        );

                        for x in 0..tile_count {
                            for y in 0..tile_count {
                                context.draw_rect(
                                    inner_origin
                                        + Point2D::new(
                                            (tile_gap + tile_size.width) * x as f32,
                                            (tile_gap + tile_size.height) * y as f32,
                                        )
                                        .to_vector(),
                                    tile_size,
                                    Color::from_hsl(130.0, 0.25, 0.5).with_alpha(opacity),
                                );
                            }
                        }
                    });
                });
            });

            context.remove_transform();
        });

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
            let rendered_chunks = context.game_loop.voxel_renderer.rendered_chunks();
            let total_chunks = context.game_loop.voxel_renderer.total_chunks();

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
Draw calls: {draw_calls}
Rendered chunks: {rendered_chunks} / {total_chunks}
Rendered vertices: {vertices}
Animation player:",
                version.1,
                version.2,
                display
                    .get_free_video_memory()
                    .map_or_else(|| String::from("unknown"), util::format_bytes),
                context.game_loop.player.position,
                chunk.x,
                chunk.y,
                1.0 / delta.as_secs_f32(),
                delta.as_secs_f32() * 1000.0,
                context.game_loop.ticks,
                context
                    .game_loop
                    .player
                    .looking_at
                    .and_then(|result| context
                        .game_loop
                        .game
                        .chunk_manager()
                        .get_block(result.position)
                        .map(|b| format!(
                            "{} (at {})",
                            if b == 1 { "dirt" } else { "grass" },
                            result.hit_side
                        )))
                    .unwrap_or_else(|| String::from("nothing")),
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
                        context.draw_text(bounds.origin, "default", text, 18.0, Color::WHITE);
                    });
                });
            });

            let mut offset = 0.0;

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
                                    bounds.origin,
                                    "default",
                                    text,
                                    18.0,
                                    Color::WHITE,
                                );

                                context.draw_rect(
                                    root.origin + Point2D::new(0.0, text_size.height + 4.0).into(),
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

                offset += text_size.height + 8.0;
            }
        }

        context.ui(|context, bounds| {
            context.fill(BG_COLOR.with_alpha(animation_progress));

            let measured = context
                .measure_text("default_bold", "Meralus", 64.0)
                .unwrap();
            let text_pos = Point2D::from_tuple(((bounds.size - measured) / 2.0).to_tuple());

            let progress_width = bounds.size.width * 0.5;
            let progress_position = (bounds.size.width - progress_width) / 2.0;
            let offset = Point2D::new(progress_position, text_pos.y + 12.0 + measured.height);

            context.bounds(
                Rect2D::new(
                    bounds.origin + offset.into(),
                    Size2D::new(progress_width, 48.0),
                ),
                |context, _| {
                    context.fill(TEXT_COLOR.with_alpha(animation_progress));

                    context.padding(2.0, |context, _| {
                        context.fill(BG_COLOR.with_alpha(animation_progress));

                        context.padding(2.0, |context, bounds| {
                            context.draw_rect(
                                bounds.origin,
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

    Application::<GameLoop>::default()
        .start()
        .expect("failed to run app");
}
