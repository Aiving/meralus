use crate::{Game, get_vertice_neighbours, mesh::Mesh, vertex_ao};

use super::{block::BlockLoader, texture::TextureLoader};
use glam::{Vec2, Vec3};
use meralus_engine::{AsValue, Color, Vertex, WindowDisplay};
use meralus_world::{Face, Faces};
use owo_colors::OwoColorize;
use std::path::Path;

#[derive(Debug)]
pub struct BlockModelFace {
    pub texture_id: usize,
    pub face: Face,
    pub cull_face: Option<Face>,
    pub uv: [Vec2; 4],
}

impl BlockModelFace {
    pub fn culled(&self, game: &Game, position: Vec3) -> bool {
        self.cull_face
            .is_some_and(|cull_face| game.block_exists(position + cull_face.as_normal()))
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
        // let normal = self.face.as_normal();

        let vertices: Vec<Vertex> = (0..4)
            .map(|i| {
                let (vertice_neighbours, extra_vertice_neighbours) = get_vertice_neighbours(
                    position,
                    vertices[i].y > 0.0,
                    vertices[i].x > 0.0,
                    vertices[i].z > 0.0,
                );

                let [side1, side2, corner] =
                    vertice_neighbours.map(|pos| game.find_block(pos).is_some());

                let ambient_occlusion = if ambient_occlusion {
                    vertex_ao(
                        side1,
                        side2,
                        corner,
                        extra_vertice_neighbours.is_some_and(|vertice_neighbours| {
                            let [side1, side2, side3] =
                                vertice_neighbours.map(|pos| game.find_block(pos).is_some());

                            (side1 || side2) && side3
                        }),
                    )
                } else {
                    1.0
                };

                let color: Vec3 = Color::WHITE.as_value();
                let color = color * ambient_occlusion;

                Vertex::from_vec(position + vertices[i], uv[i], Color::from(color))
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
    pub faces: Vec<BlockModelFace>,
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
        display: &WindowDisplay,
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
        let block = BlockLoader::load(textures, display, root.as_ref(), path)?;

        let mut faces = Vec::new();

        for element in block.elements {
            match element.faces {
                Faces::All(data) => {
                    for face in Face::ALL {
                        let texture = block.textures.get(&data.texture)?;
                        let texture_id =
                            textures.get_id(texture.1.file_stem()?.to_string_lossy())?;

                        faces.push(BlockModelFace {
                            texture_id,
                            face,
                            cull_face: Some(face),
                            uv: face.as_uv(),
                        });
                    }
                }
                Faces::Unique(face_map) => {
                    for (face, data) in face_map {
                        let texture = block.textures.get(&data.texture)?;
                        let texture_id =
                            textures.get_id(texture.1.file_stem()?.to_string_lossy())?;

                        faces.push(BlockModelFace {
                            texture_id,
                            face,
                            cull_face: Some(face),
                            uv: face.as_uv(),
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
