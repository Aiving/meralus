use crate::Face;
use macroquad::{color::Color, texture::Texture2D};

pub struct Block {
    faces: Vec<(Face, Texture2D, Option<Color>)>,
}

impl FromIterator<(Face, Texture2D, Option<Color>)> for Block {
    fn from_iter<I: IntoIterator<Item = (Face, Texture2D, Option<Color>)>>(iter: I) -> Self {
        let faces = iter.into_iter().collect();

        Self { faces }
    }
}

impl Block {
    #[must_use]
    pub fn get_face_textures(&self, face: Face) -> Vec<(Texture2D, Option<Color>)> {
        self.faces
            .iter()
            .filter_map(|(f, t, c)| {
                if *f == face {
                    Some((t.clone(), *c))
                } else {
                    None
                }
            })
            .collect()
    }
}
