use std::fmt;

use macroquad::math::{Vec2, Vec3, vec2, vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
}
