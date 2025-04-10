use crate::Face;
use macroquad::{
    camera::{Camera, Camera3D},
    color::{BLACK, Color, hsl_to_rgb},
    input::{KeyCode, is_key_down},
    math::{Vec2, Vec3, Vec4, vec2, vec3},
    miniquad::window::screen_size,
    ui::Vertex,
};

const AMBIENT_OCCLUSION_VALUES: [f32; 4] = [0.1, 0.25, 0.5, 1.0];

#[must_use]
pub fn get_movement_direction() -> Vec3 {
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

#[must_use]
pub fn get_rotation_directions(yaw: f32, pitch: f32) -> (Vec3, Vec3, Vec3) {
    let front: Vec3 = vec3(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    )
    .normalize();

    let right = front.cross(Vec3::Y).normalize();

    (front, right, right.cross(front).normalize())
}

#[must_use]
pub fn get_vertice_neighbours(
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

#[must_use]
pub fn vertex_ao(side1: bool, side2: bool, corner: bool) -> f32 {
    AMBIENT_OCCLUSION_VALUES[if side1 && side2 {
        0
    } else {
        3 - (usize::from(side1) + usize::from(side2) + usize::from(corner))
    }]
}

pub trait VertexExt {
    #[must_use]
    fn with_normal(self, normal: Vec4) -> Self;
}

impl VertexExt for Vertex {
    fn with_normal(mut self, normal: Vec4) -> Self {
        self.normal = normal;

        self
    }
}

pub trait Vec3Ext {
    fn as_color(&self) -> Color;
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

pub trait CameraExt {
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
