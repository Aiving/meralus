use meralus_engine::Color;
use meralus_world::Face;

pub struct Block {
    faces: Vec<(Face, usize, Option<Color>)>,
}

impl FromIterator<(Face, usize, Option<Color>)> for Block {
    fn from_iter<I: IntoIterator<Item = (Face, usize, Option<Color>)>>(iter: I) -> Self {
        let faces = iter.into_iter().collect();

        Self { faces }
    }
}

impl Block {
    #[must_use]
    pub fn get_face_textures(&self, face: Face) -> Vec<(usize, Option<Color>)> {
        self.faces
            .iter()
            .filter(|&&(f, ..)| f == face)
            .map(|&(_, t, c)| (t, c))
            .collect()
    }
}
