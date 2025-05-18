use glam::IVec2;
use meralus_engine::Vertex;
use meralus_world::Face;

#[derive(Debug)]
pub struct Mesh {
    pub origin: IVec2,
    pub face: Face,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub texture_id: usize,
}

impl Mesh {
    pub const fn empty(face: Face) -> Self {
        Self {
            origin: IVec2::ZERO,
            face,
            vertices: Vec::new(),
            indices: Vec::new(),
            texture_id: 0,
        }
    }
}
