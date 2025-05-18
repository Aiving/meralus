use glam::{DVec3, Vec3};
use meralus_world::Face;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayCastResult {
    pub position: Vec3,
    pub hit_type: HitType,
    pub hit_side: Face,
    pub hit_vec: DVec3,
}

impl RayCastResult {
    pub const fn new(hit_type: HitType, hit_vec: DVec3, hit_side: Face, position: Vec3) -> Self {
        Self {
            position,
            hit_type,
            hit_side,
            hit_vec,
        }
    }

    pub const fn new2(hit_vec: DVec3, hit_side: Face) -> Self {
        Self::new(HitType::Block, hit_vec, hit_side, Vec3::ZERO)
    }

    pub const fn new3(hit_vec: DVec3, hit_side: Face, position: Vec3) -> Self {
        Self::new(HitType::Block, hit_vec, hit_side, position)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HitType {
    None,
    Block,
}
