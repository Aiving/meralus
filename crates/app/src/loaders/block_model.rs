use std::path::Path;

use glam::{Vec2, Vec3};
use meralus_shared::Color;
use meralus_world::{Face, Faces};
use owo_colors::OwoColorize;

use super::{block::BlockManager, texture::TextureLoader};
use crate::Game;

#[derive(Debug)]
pub struct BlockModelFace {
    pub texture_id: usize,
    pub face: Face,
    pub cull_face: Option<Face>,
    pub tint: bool,
    pub uv: [Vec2; 4],
    pub overlay_uv: Option<[Vec2; 4]>,
    pub overlay_color: Option<Color>,
}

impl BlockModelFace {
    pub fn culled(&self, game: &Game, position: Vec3) -> bool {
        self.cull_face.is_some_and(|cull_face| {
            game.chunk_manager()
                .contains_block(position + cull_face.as_normal().as_vec3())
        })
    }
}

#[derive(Debug)]
pub struct BakedBlockModel {
    pub name: String,
    pub ambient_occlusion: bool,
    pub faces: Vec<BlockModelFace>,
}

#[derive(Debug, Default)]
pub struct BakedBlockModelLoader {
    models: Vec<BakedBlockModel>,
}

impl BakedBlockModelLoader {
    #[allow(clippy::missing_const_for_fn)] // for MSRV compatibility
    pub fn count(&self) -> usize {
        self.models.len()
    }

    pub fn get(&self, value: usize) -> Option<&BakedBlockModel> {
        self.models.get(value)
    }

    pub fn load<P: AsRef<Path>, R: AsRef<Path>>(
        &mut self,
        textures: &mut TextureLoader,
        root: R,
        path: P,
    ) -> Option<&BakedBlockModel> {
        let path = path.as_ref();

        println!(
            "[{:18}] Loading model at {}",
            "INFO/ModelLoader".bright_green(),
            path.display().bright_blue().bold()
        );

        let name = path.file_stem()?.to_string_lossy();
        let block = BlockManager::load(textures, root.as_ref(), path)?;

        let mut faces = Vec::new();

        for element in block.elements {
            match element.faces {
                Faces::All(data) => {
                    for face in Face::ALL {
                        faces.push({
                            let texture = block.textures.get(&data.texture).unwrap();
                            let (offset, scale) = textures
                                .get_texture(texture.1.file_stem().unwrap().to_string_lossy())
                                .unwrap();

                            let uv = face.as_uv().map(|uv| offset + uv * (scale));

                            BlockModelFace {
                                texture_id: 0,
                                face,
                                cull_face: Some(face),
                                uv,
                                tint: data.tint,
                                overlay_uv: None,
                                overlay_color: None,
                            }
                        });
                    }
                }
                Faces::Unique(face_map) => {
                    for (face, data) in face_map {
                        faces.push({
                            let texture = block.textures.get(&data.texture).unwrap();
                            let (offset, scale) = textures
                                .get_texture(texture.1.file_stem().unwrap().to_string_lossy())
                                .unwrap();

                            let uv = face.as_uv().map(|uv| offset + uv * (scale));

                            BlockModelFace {
                                texture_id: 0,
                                face,
                                cull_face: Some(face),
                                uv,
                                tint: data.tint,
                                overlay_uv: None,
                                overlay_color: None,
                            }
                        });
                    }
                }
            }
        }

        self.models.push(BakedBlockModel {
            name: name.to_string(),
            ambient_occlusion: block.ambient_occlusion,
            faces,
        });

        self.models.last()
    }
}
