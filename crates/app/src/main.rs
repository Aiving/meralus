#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal,
    clippy::missing_panics_doc
)]

mod block;
mod face;
mod game;
mod player;
mod transform;
mod util;

pub use self::{
    block::Block,
    face::{Axis, Face},
    game::{BackedFace, Game, GameState},
    player::PlayerController,
    transform::Transform,
    util::{
        CameraExt, Vec3Ext, VertexExt, get_movement_direction, get_rotation_directions,
        get_vertice_neighbours, raycast, vertex_ao,
    },
};
use macroquad::{miniquad::gl, prelude::*};
use std::path::PathBuf;

const DEBUG_FACES: bool = true;

fn conf() -> Conf {
    Conf {
        window_title: String::from("Macroquad"),
        window_width: 1260,
        window_height: 768,
        ..Default::default()
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
    macroquad::Window::from_config(conf(), app(Game::new(12723, 0..1, 0..1)));
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

fn debug_current_block_faces(game: &Game, state: &GameState, vertices: &[Vertex]) {
    if let Some((_, block_position)) = state.current_block {
        let mut width = 0.0f32;
        let mut height = 24.0f32;

        for vertex in vertices {
            let position = vertex.position - block_position.as_vec3();
            let ([side1, side2, corner], _) = get_vertice_neighbours(
                block_position.as_vec3(),
                position.y > 0.0,
                position.x > 0.0,
                position.z > 0.0,
            );

            let text = format!(
                "{:<5} {:<6} {:<5} ({}) AO: {:<4} (side1[{side1}]: {:?}, side2[{side2}]: {:?}, corner[{corner}]: {:?})",
                Face::from_axis_value(Axis::X, position.x),
                Face::from_axis_value(Axis::Y, position.y),
                Face::from_axis_value(Axis::Z, position.z),
                position,
                vertex.normal.w,
                game.find_block(side1),
                game.find_block(side2),
                game.find_block(corner),
            );

            let measured = measure_text(&text, None, 16, 1.0);

            width = width.max(measured.width);
            height += measured.height;
        }

        draw_rectangle(
            screen_width() - width - 24.0,
            screen_height() - height,
            width + 24.0,
            height,
            BLACK,
        );

        for (i, vertex) in vertices.iter().enumerate() {
            let i = i as f32;
            let position = vertex.position - block_position.as_vec3();
            let ([side1, side2, corner], _) = get_vertice_neighbours(
                block_position.as_vec3(),
                position.y > 0.0,
                position.x > 0.0,
                position.z > 0.0,
            );
            let text = format!(
                "{:<5} {:<6} {:<5} ({}) AO: {:<4} (side1[{side1}]: {:?}, side2[{side2}]: {:?}, corner[{corner}]: {:?})",
                Face::from_axis_value(Axis::X, position.x),
                Face::from_axis_value(Axis::Y, position.y),
                Face::from_axis_value(Axis::Z, position.z),
                position,
                vertex.normal.w,
                game.find_block(side1),
                game.find_block(side2),
                game.find_block(corner),
            );

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

    let mut looking_at = None;

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
            player.handle_mouse(
                &mut looking_at,
                &game,
                mouse_position - last_mouse_position,
                delta,
            );
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

        if let Some(position) = looking_at {
            draw_cube_wires(position + (Vec3::ONE / 2.0), Vec3::ONE * 1.1, GRAY);
        }

        // Back to screen space, render some text

        set_default_camera();

        if wireframe {
            unsafe {
                gl::glPolygonMode(gl::GL_FRONT_AND_BACK, gl::GL_FILL);
            }
        }

        if DEBUG_FACES {
            debug_current_block_faces(&game, &state, &vertices);

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
                player.yaw / PlayerController::LOOK_SPEED,
                player.pitch / -PlayerController::LOOK_SPEED,
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
