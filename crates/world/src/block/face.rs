use glam::{IVec3, U16Vec3, Vec2, Vec3, ivec3, vec2, vec3};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Front,
    Back,
}

impl fmt::Display for Face {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Top => f.write_str("Top"),
            Self::Bottom => f.write_str("Bottom"),
            Self::Left => f.write_str("Left"),
            Self::Right => f.write_str("Right"),
            Self::Front => f.write_str("Front"),
            Self::Back => f.write_str("Back"),
        }
    }
}

pub enum Axis {
    X,
    Y,
    Z,
}

impl Face {
    pub const ALL: [Self; 6] = [
        Self::Top,
        Self::Bottom,
        Self::Left,
        Self::Right,
        Self::Front,
        Self::Back,
    ];

    pub const VERTICES: [Vec3; 8] = [
        vec3(0.0, 0.0, 1.0), // 0 LEFT  BOTTOM FRONT
        vec3(1.0, 0.0, 1.0), // 1 RIGHT BOTTOM FRONT
        vec3(0.0, 1.0, 1.0), // 2 LEFT  TOP    FRONT
        vec3(1.0, 1.0, 1.0), // 3 RIGHT TOP    FRONT
        vec3(0.0, 0.0, 0.0), // 4 LEFT  BOTTOM BACK
        vec3(1.0, 0.0, 0.0), // 5 RIGHT BOTTOM BACK
        vec3(0.0, 1.0, 0.0), // 6 LEFT  TOP    BACK
        vec3(1.0, 1.0, 0.0), // 7 RIGHT TOP    BACK
    ];

    pub const fn normal_index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Bottom => 2,
            Self::Top => 3,
            Self::Front => 4,
            Self::Back => 5,
        }
    }

    pub const fn world_to_sample(&self, axis: i32, x: i32, y: i32) -> IVec3 {
        match self {
            Self::Top => ivec3(x, axis + 1, y),
            Self::Bottom => ivec3(x, axis, y),
            Self::Left => ivec3(axis, y, x),
            Self::Right => ivec3(axis + 1, y, x),
            Self::Front => ivec3(x, y, axis),
            Self::Back => ivec3(x, y, axis + 1),
        }
    }

    pub const fn reverse_order(&self) -> bool {
        matches!(self, Self::Top | Self::Right | Self::Front)
    }

    #[must_use]
    pub fn from_axis_value(axis: Axis, value: f32) -> Self {
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

    #[must_use]
    pub const fn as_vertices(self) -> [Vec3; 4] {
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

    #[must_use]
    pub const fn as_full_vertices(self) -> [Vec3; 6] {
        match self {
            Self::Top => [
                Self::VERTICES[2],
                Self::VERTICES[6],
                Self::VERTICES[7],
                Self::VERTICES[7],
                Self::VERTICES[3],
                Self::VERTICES[2],
            ],
            Self::Bottom => [
                Self::VERTICES[1],
                Self::VERTICES[5],
                Self::VERTICES[4],
                Self::VERTICES[4],
                Self::VERTICES[0],
                Self::VERTICES[1],
            ],
            Self::Left => [
                Self::VERTICES[4],
                Self::VERTICES[6],
                Self::VERTICES[2],
                Self::VERTICES[2],
                Self::VERTICES[0],
                Self::VERTICES[4],
            ],
            Self::Right => [
                Self::VERTICES[1],
                Self::VERTICES[3],
                Self::VERTICES[7],
                Self::VERTICES[7],
                Self::VERTICES[5],
                Self::VERTICES[1],
            ],
            Self::Front => [
                Self::VERTICES[1],
                Self::VERTICES[3],
                Self::VERTICES[2],
                Self::VERTICES[2],
                Self::VERTICES[0],
                Self::VERTICES[1],
            ],
            Self::Back => [
                Self::VERTICES[5],
                Self::VERTICES[7],
                Self::VERTICES[6],
                Self::VERTICES[6],
                Self::VERTICES[4],
                Self::VERTICES[5],
            ],
        }
    }

    #[must_use]
    pub const fn as_uv(self) -> [Vec2; 4] {
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

    #[must_use]
    pub const fn as_normal(self) -> Vec3 {
        match self {
            Self::Top => Vec3::Y,
            Self::Bottom => Vec3::NEG_Y,
            Self::Right => Vec3::X,
            Self::Left => Vec3::NEG_X,
            Self::Front => Vec3::Z,
            Self::Back => Vec3::NEG_Z,
        }
    }

    #[must_use]
    pub const fn add_position(self, mut position: U16Vec3) -> U16Vec3 {
        match self {
            Self::Top => position.y += 1,
            Self::Bottom => position.y = position.y.saturating_sub(1),
            Self::Right => position.x += 1,
            Self::Left => position.x = position.x.saturating_sub(1),
            Self::Front => position.z += 1,
            Self::Back => position.z = position.z.saturating_sub(1),
        }

        position
    }
}
