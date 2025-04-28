use crate::{Game, mesh::Mesh, vertex_ao};

use super::{block::BlockLoader, texture::TextureLoader};
use glam::{Vec2, Vec3};
use meralus_engine::{AsValue, Color, Vertex};
use meralus_world::{Face, Faces};
use owo_colors::OwoColorize;
use std::{collections::HashMap, path::Path};

#[derive(Debug)]
pub struct BlockModelFace {
    pub texture_id: usize,
    pub face: Face,
    pub cull_face: Option<Face>,
    pub uv: [Vec2; 4],
    pub overlay_uv: Option<[Vec2; 4]>,
    pub overlay_color: Option<Color>,
}

impl BlockModelFace {
    pub fn culled(&self, game: &Game, position: Vec3) -> bool {
        self.cull_face
            .is_some_and(|cull_face| game.block_exists(position + cull_face.as_normal().as_vec3()))
    }

    pub fn as_mesh(
        &self,
        game: &Game,
        position: Vec3,
        color: Option<Color>,
        ambient_occlusion: bool,
    ) -> Mesh {
        let vertices = self.face.as_vertices();
        let uv = self.uv;
        let vertice_corners = self.face.as_vertice_corners();

        let vertices: Vec<Vertex> = (0..4)
            .map(|i| {
                let [side1, side2, corner] = vertice_corners[i]
                    .get_neighbours(self.face)
                    .map(|neighbour| game.block_exists(position + neighbour.as_vec3()));

                let ambient_occlusion = if ambient_occlusion {
                    vertex_ao(side1, side2, corner)
                } else {
                    1.0
                };

                let color: Vec3 = Color::WHITE.as_value();
                let color = color * ambient_occlusion;

                Vertex::from_vec(
                    position + vertices[i],
                    uv[i],
                    Color::from(color),
                    None,
                    None,
                    false,
                )
            })
            .collect();

        // let indices = if vertices[1].normal.w + vertices[3].normal.w
        //     > vertices[0].normal.w + vertices[2].normal.w
        // {
        //     // FLIP!
        //     vec![3, 2, 1, 1, 0, 3]
        // } else {
        //     vec![0, 1, 2, 2, 3, 0]
        // };

        let mut mesh = Mesh {
            vertices,
            indices: vec![0, 1, 2, 2, 3, 0],
            texture_id: self.texture_id,
        };

        if let Some(color) = color.as_ref().map(AsValue::<Vec3>::as_value) {
            for vertex in &mut mesh.vertices {
                let color0: Vec3 = vertex.color.as_value();

                vertex.color = Color::from(color0 * color);
            }
        }

        mesh
    }
}

#[derive(Debug)]
pub struct BlockModel {
    pub name: String,
    pub ambient_occlusion: bool,
    pub faces: HashMap<Face, BlockModelFace>,
}

#[derive(Debug, Default)]
pub struct BlockModelLoader {
    models: Vec<BlockModel>,
}

impl BlockModelLoader {
    pub const fn count(&self) -> usize {
        self.models.len()
    }

    pub fn get(&self, value: usize) -> Option<&BlockModel> {
        self.models.get(value)
    }

    pub fn load<P: AsRef<Path>, R: AsRef<Path>>(
        &mut self,
        textures: &mut TextureLoader,
        root: R,
        path: P,
    ) -> Option<&BlockModel> {
        let path = path.as_ref();

        println!(
            "[{:18}] Loading model at {}",
            "INFO/ModelLoader".bright_green(),
            path.display().bright_blue().bold()
        );

        let name = path.file_stem()?.to_string_lossy();
        let block = BlockLoader::load(textures, root.as_ref(), path)?;

        let mut faces = <HashMap<Face, BlockModelFace>>::new();

        for element in block.elements {
            match element.faces {
                Faces::All(data) => {
                    for face in Face::ALL {
                        faces
                            .entry(face)
                            .and_modify(|model| {
                                let texture = block.textures.get(&data.texture).unwrap();
                                let (offset, scale) = textures
                                    .get_texture(texture.1.file_stem().unwrap().to_string_lossy())
                                    .unwrap();

                                let overlay_uv = face.as_uv().map(|uv| offset + uv * (scale));

                                model.overlay_uv = Some(overlay_uv);
                            })
                            .or_insert_with(|| {
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
                                    overlay_uv: None,
                                    overlay_color: None,
                                }
                            });
                    }
                }
                Faces::Unique(face_map) => {
                    for (face, data) in face_map {
                        faces
                            .entry(face)
                            .and_modify(|model| {
                                let texture = block.textures.get(&data.texture).unwrap();
                                let (offset, scale) = textures
                                    .get_texture(texture.1.file_stem().unwrap().to_string_lossy())
                                    .unwrap();

                                let overlay_uv = face.as_uv().map(|uv| offset + uv * (scale));

                                model.overlay_uv = Some(overlay_uv);
                            })
                            .or_insert_with(|| {
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
                                    overlay_uv: None,
                                    overlay_color: None,
                                }
                            });
                    }
                }
            }
        }

        self.models.push(BlockModel {
            name: name.to_string(),
            ambient_occlusion: block.ambient_occlusion,
            faces,
        });

        self.models.last()
    }
}
