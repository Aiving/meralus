use crate::{Camera, Game, KeyboardController};
use glam::{IVec3, Vec2, Vec3, vec2, vec3};
use meralus_engine::{
    KeyCode,
    glium::{buffer::ReadError, pixel_buffer::PixelBuffer},
};
use meralus_shared::Color;
use meralus_world::Face;

const AMBIENT_OCCLUSION_VALUES: [f32; 4] = [0.1, 0.25, 0.5, 1.0];

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

pub fn raycast(game: &Game, origin: IVec3, direction: Vec3, mut radius: f32) -> Option<Vec3> {
    // From "A Fast Voxel Traversal Algorithm for Ray Tracing"
    // by John Amanatides and Andrew Woo, 1987
    // <http://www.cse.yorku.ca/~amana/research/grid.pdf>
    // <http://citeseer.ist.psu.edu/viewdoc/summary?doi=10.1.1.42.3443>
    // Extensions to the described algorithm:
    //   • Imposed a distance limit.
    //   • The face passed through to reach the current cube is provided to
    //     the callback.

    // The foundation of this algorithm is a parameterized representation of
    // the provided ray,
    //                    origin + t * direction,
    // except that t is not actually stored; rather, at any given point in the
    // traversal, we keep track of the *greater* t values which we would have
    // if we took a step sufficient to cross a cube boundary along that axis
    // (i.e. change the integer part of the coordinate) in the variables
    // t_max_x, t_max_y, and t_max_z.

    // Cube containing origin point.
    let Vec3 {
        mut x,
        mut y,
        mut z,
    } = origin.as_vec3();
    // Break out direction vector.
    let Vec3 {
        x: dx,
        y: dy,
        z: dz,
    } = direction;
    // Direction to increment x,y,z when stepping.
    let [step_x, step_y, step_z] = [dx.signum(), dy.signum(), dz.signum()];
    // See description above. The initial values depend on the fractional
    // part of the origin.
    let [mut t_max_x, mut t_max_y, mut t_max_z] =
        [intbound(x, dx), intbound(y, dy), intbound(z, dz)];
    // The change in t when taking a step (always positive).
    let [t_delta_x, t_delta_y, t_delta_z] = [step_x / dx, step_y / dy, step_z / dz];
    // Buffer for reporting faces to the callback.
    let mut face = Vec3::ZERO;

    // Avoids an infinite loop.
    if dx == 0.0 && dy == 0.0 && dz == 0.0 {
        return None;
    }
    //   throw new RangeError("Raycast in zero direction!");

    // Rescale from units of 1 cube-edge to units of 'direction' so we can
    // compare with 't'.
    radius /= dz.mul_add(dz, dx.mul_add(dx, dy.powi(2))).sqrt();

    let bounds = game.chunk_manager.surface_size().as_vec3();
    let mut block = None;

    while (if step_x > 0.0 { x < bounds.x } else { x >= 0.0 })
        && (if step_y > 0.0 { y < bounds.y } else { y >= 0.0 })
        && (if step_z > 0.0 { z < bounds.z } else { z >= 0.0 })
    {
        // Invoke the callback, unless we are not *yet* within the bounds of the
        // world.
        if (!(x < 0.0 || y < 0.0 || z < 0.0 || x >= bounds.x || y >= bounds.y || z >= bounds.z))
            && (game.chunk_manager.contains_block(vec3(x, y, z)))
        {
            block = Some(vec3(x, y, z));

            break;
        }

        // t_max_x stores the t-value at which we cross a cube boundary along the
        // X axis, and similarly for Y and Z. Therefore, choosing the least tMax
        // chooses the closest cube boundary. Only the first case of the four
        // has been commented in detail.
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                if t_max_x > radius {
                    break;
                }

                // Update which cube we are now in.
                x += step_x;
                // Adjust t_max_x to the next X-oriented boundary crossing.
                t_max_x += t_delta_x;

                // Record the normal vector of the cube face we entered.
                face.x = -step_x;
                face.y = 0.0;
                face.z = 0.0;
            } else {
                if t_max_z > radius {
                    break;
                }

                z += step_z;
                t_max_z += t_delta_z;

                face.x = 0.0;
                face.y = 0.0;
                face.z = -step_z;
            }
        } else if t_max_y < t_max_z {
            if t_max_y > radius {
                break;
            }

            y += step_y;
            t_max_y += t_delta_y;

            face.x = 0.0;
            face.y = -step_y;
            face.z = 0.0;
        } else {
            // Identical to the second case, repeated for simplicity in
            // the conditionals.
            if t_max_z > radius {
                break;
            }

            z += step_z;
            t_max_z += t_delta_z;

            face.x = 0.0;
            face.y = 0.0;
            face.z = -step_z;
        }
    }

    block
}

fn intbound(s: f32, ds: f32) -> f32 {
    // Find the smallest positive t such that s+t*ds is an integer.
    if ds < 0.0 {
        intbound(-s, -ds)
    } else {
        let s = s % 1.0;

        // problem is now s+t*ds = 1
        (1.0 - s) / ds
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
