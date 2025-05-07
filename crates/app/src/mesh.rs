use glam::IVec2;
use meralus_engine::Vertex;

#[derive(Debug, Default)]
pub struct Mesh {
    pub origin: IVec2,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub texture_id: usize,
}

impl Mesh {
    pub const EMPTY: Self = Self {
        origin: IVec2::ZERO,
        vertices: Vec::new(),
        indices: Vec::new(),
        texture_id: 0,
    };
}
