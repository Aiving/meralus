use glam::{DVec3, Vec2, Vec3, vec2, vec3};
use glamour::ToRaw;
use glium::{buffer::ReadError, pixel_buffer::PixelBuffer};
use meralus_engine::KeyCode;
use meralus_shared::{Color, Cube3D};
use meralus_world::Face;

use crate::{Camera, KeyboardController, renderers::Line};

const AMBIENT_OCCLUSION_VALUES: [f32; 4] = [0.4, 0.55, 0.75, 1.0];

#[must_use]
pub fn get_movement_direction(keyboard: &KeyboardController) -> Vec3 {
    let mut direction = Vec3::ZERO;

    if keyboard.is_key_pressed(KeyCode::KeyW) {
        direction.z += 1.;
    }

    if keyboard.is_key_pressed(KeyCode::KeyS) {
        direction.z -= 1.;
    }

    if keyboard.is_key_pressed(KeyCode::KeyA) {
        direction.x -= 1.;
    }

    if keyboard.is_key_pressed(KeyCode::KeyD) {
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
#[allow(clippy::fn_params_excessive_bools)]
pub fn vertex_ao(side1: bool, side2: bool, corner: bool) -> f32 {
    AMBIENT_OCCLUSION_VALUES[if side1 && side2 {
        0
    } else {
        3 - (usize::from(side1) + usize::from(side2) + usize::from(corner))
    }]
}

pub trait AsColor {
    fn as_color(&self) -> Color;
}

impl AsColor for Face {
    fn as_color(&self) -> Color {
        match self {
            Self::Top => Color::RED,
            Self::Bottom => Color::GREEN,
            Self::Left => Color::BLUE,
            Self::Right => Color::YELLOW,
            Self::Front => Color::BROWN,
            Self::Back => Color::PURPLE,
        }
    }
}

impl AsColor for Vec3 {
    fn as_color(&self) -> Color {
        for (pos, vertice) in Face::VERTICES.iter().enumerate() {
            if self == vertice {
                return Color::from_hsl(pos as f32 / 8.0, 1.0, 0.5);
            }
        }

        Color::BLACK
    }
}

pub trait BufferExt {
    fn read_flatten(&self) -> Result<Vec<u8>, ReadError>;
}

impl BufferExt for PixelBuffer<(u8, u8, u8, u8)> {
    fn read_flatten(&self) -> Result<Vec<u8>, ReadError> {
        let mut pixels = Vec::with_capacity(self.len() * 4);
        let buffer = self.read()?;

        for (a, b, c, d) in buffer {
            pixels.push(a);
            pixels.push(b);
            pixels.push(c);
            pixels.push(d);
        }

        Ok(pixels)
    }
}

pub trait CameraExt {
    fn unproject_position(&self, width: f32, height: f32, position: Vec3) -> Option<(Vec2, f32)>;
}

impl CameraExt for Camera {
    fn unproject_position(&self, width: f32, height: f32, position: Vec3) -> Option<(Vec2, f32)> {
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

pub trait VecExt<T>: Sized {
    fn get_intermediate_with_x_value(&self, vec: Self, x: T) -> Option<Self>;
    fn get_intermediate_with_y_value(&self, vec: Self, y: T) -> Option<Self>;
    fn get_intermediate_with_z_value(&self, vec: Self, z: T) -> Option<Self>;
}

impl VecExt<f64> for DVec3 {
    fn get_intermediate_with_x_value(&self, vec: Self, x: f64) -> Option<Self> {
        let d0 = vec.x - self.x;
        let d1 = vec.y - self.y;
        let d2 = vec.z - self.z;

        if d0 * d0 < 0.0000001 {
            None
        } else {
            let d3 = (x - self.x) / d0;

            if (0.0..=1.0).contains(&d3) {
                Some(Self::new(
                    d0.mul_add(d3, self.x),
                    d1.mul_add(d3, self.y),
                    d2.mul_add(d3, self.z),
                ))
            } else {
                None
            }
        }
    }

    fn get_intermediate_with_y_value(&self, vec: Self, y: f64) -> Option<Self> {
        let d0 = vec.x - self.x;
        let d1 = vec.y - self.y;
        let d2 = vec.z - self.z;

        if d1 * d1 < 1.0000000116860974E-7 {
            None
        } else {
            let d3 = (y - self.y) / d1;

            if (0.0..=1.0).contains(&d3) {
                Some(Self::new(
                    d0.mul_add(d3, self.x),
                    d1.mul_add(d3, self.y),
                    d2.mul_add(d3, self.z),
                ))
            } else {
                None
            }
        }
    }

    fn get_intermediate_with_z_value(&self, vec: Self, z: f64) -> Option<Self> {
        let d0 = vec.x - self.x;
        let d1 = vec.y - self.y;
        let d2 = vec.z - self.z;

        if d2 * d2 < 1.0000000116860974E-7 {
            None
        } else {
            let d3 = (z - self.x) / d2;

            if (0.0..=1.0).contains(&d3) {
                Some(Self::new(
                    d0.mul_add(d3, self.x),
                    d1.mul_add(d3, self.y),
                    d2.mul_add(d3, self.z),
                ))
            } else {
                None
            }
        }
    }
}

pub const SIZE_CAP: f32 = 960.0;

pub fn format_bytes(bytes: usize) -> String {
    let mut value = bytes as f32;

    for suffix in ["B", "kB", "MB"] {
        if value > SIZE_CAP {
            value /= 1024.0;
        } else {
            return format!("{value:.2}{suffix}");
        }
    }

    format!("{value:.2}GB")
}

pub fn cube_outline(Cube3D { origin, size }: Cube3D) -> [Line; 12] {
    [
        [[0.0, 0.0, 0.0], [0.0, size.height, 0.0]],
        [[size.width, 0.0, 0.0], [size.width, size.height, 0.0]],
        [[0.0, 0.0, size.depth], [0.0, size.height, size.depth]],
        [[size.width, 0.0, size.depth], [
            size.width,
            size.height,
            size.depth,
        ]],
        [[0.0, 0.0, 0.0], [size.width, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, size.depth]],
        [[size.width, 0.0, 0.0], [size.width, 0.0, size.depth]],
        [[0.0, 0.0, size.depth], [size.width, 0.0, size.depth]],
        [[0.0, size.height, 0.0], [size.width, size.height, 0.0]],
        [[0.0, size.height, 0.0], [0.0, size.height, size.depth]],
        [[size.width, size.height, 0.0], [
            size.width,
            size.height,
            size.depth,
        ]],
        [[0.0, size.height, size.depth], [
            size.width,
            size.height,
            size.depth,
        ]],
    ]
    .map(|[start, end]| {
        Line::new(
            origin.to_raw() + Vec3::from_array(start),
            origin.to_raw() + Vec3::from_array(end),
            Color::BLUE,
        )
    })
}
