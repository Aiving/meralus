use meralus_engine::Vertex;

#[derive(Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub texture_id: usize,
}

impl Mesh {
    pub const EMPTY: Self = Self {
        vertices: Vec::new(),
        indices: Vec::new(),
        texture_id: 0,
    };
}
