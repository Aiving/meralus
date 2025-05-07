use super::Shader;
use crate::{BLENDING, mesh::Mesh};
use glam::{IVec2, Mat4};
use meralus_engine::{
    Vertex, WindowDisplay,
    glium::{
        BackfaceCullingMode, Depth, DepthTest, DrawParameters, Frame, PolygonMode, Program,
        Surface, Texture2d, VertexBuffer,
        index::{NoIndices, PrimitiveType},
        uniform,
        uniforms::Sampler,
    },
};
use owo_colors::OwoColorize;

struct VoxelShader;

impl Shader for VoxelShader {
    const VERTEX: &str = include_str!("../../resources/shaders/voxel.vs");
    const FRAGMENT: &str = include_str!("../../resources/shaders/voxel.fs");
}

pub struct VoxelRenderer {
    shader: Program,
    draws: Vec<(IVec2, VertexBuffer<Vertex>, usize)>,
    vertices: usize,
    draw_calls: usize,
}

impl VoxelRenderer {
    pub fn new(display: &WindowDisplay, world_mesh: Vec<[Mesh; 6]>) -> Self {
        let mut draws = Vec::new();
        let mut vertices = 0;
        let mut draw_calls = 0;

        for meshes in world_mesh {
            for mesh in meshes {
                draws.push((
                    mesh.origin,
                    VertexBuffer::new(display, &mesh.vertices).unwrap(),
                    mesh.texture_id,
                ));

                vertices += mesh.vertices.len();
                draw_calls += 1;
            }
        }

        println!(
            "[{:18}] All DrawCall's for OpenGL created",
            "INFO/Rendering".bright_green(),
        );

        Self {
            shader: VoxelShader::program(display),
            draws,
            vertices,
            draw_calls,
        }
    }

    pub const fn get_debug_info(&self) -> (usize, usize) {
        (self.draw_calls, self.vertices)
    }

    pub fn render_with_params(
        &self,
        frame: &mut Frame,
        matrix: Mat4,
        atlas: Sampler<'_, Texture2d>,
        params: &DrawParameters,
    ) {
        for (origin, vertex_buffer, _) in &self.draws {
            let uniforms = uniform! {
                origin: origin.to_array(),
                matrix: matrix.to_cols_array_2d(),
                tex: atlas,
                with_tex: true,
            };

            frame
                .draw(
                    vertex_buffer,
                    NoIndices(PrimitiveType::TrianglesList),
                    &self.shader,
                    &uniforms,
                    params,
                )
                .expect("failed to draw!");
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        matrix: Mat4,
        atlas: Sampler<'_, Texture2d>,
        wireframe: bool,
    ) {
        self.render_with_params(
            frame,
            matrix,
            atlas,
            &DrawParameters {
                depth: Depth {
                    test: DepthTest::IfLessOrEqual,
                    write: true,
                    ..Depth::default()
                },
                backface_culling: BackfaceCullingMode::CullCounterClockwise,
                polygon_mode: if wireframe {
                    PolygonMode::Line
                } else {
                    PolygonMode::Fill
                },
                blend: BLENDING,
                ..DrawParameters::default()
            },
        );
    }
}
