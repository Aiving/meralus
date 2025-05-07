use super::Shader;
use crate::{BLENDING, impl_vertex};
use glam::{Mat4, Vec2, Vec3};
use meralus_engine::{
    WindowDisplay,
    glium::{
        DrawParameters, Frame, Program, Surface, VertexBuffer,
        index::{NoIndices, PrimitiveType},
        uniform,
    },
};
use meralus_shared::Color;

struct ShapeShader;

impl Shader for ShapeShader {
    const VERTEX: &str = include_str!("../../resources/shaders/shape.vs");
    const FRAGMENT: &str = include_str!("../../resources/shaders/shape.fs");
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeVertex {
    pub position: Vec3,
    pub color: Color,
}

impl_vertex! {
    ShapeVertex {
        position: [f32; 3],
        color: [u8; 4]
    }
}

pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Color,
}

impl Line {
    pub const fn new(start: Vec3, end: Vec3, color: Color) -> Self {
        Self { start, end, color }
    }

    pub const fn as_vertices(&self) -> [ShapeVertex; 2] {
        [
            ShapeVertex {
                position: self.start,
                color: self.color,
            },
            ShapeVertex {
                position: self.end,
                color: self.color,
            },
        ]
    }
}

pub struct Rectangle {
    pub position: Vec2,
    pub size: Vec2,
    pub color: Color,
}

impl Rectangle {
    pub const fn new(x: f32, y: f32, width: f32, height: f32, color: Color) -> Self {
        Self {
            position: Vec2::new(x, y),
            size: Vec2::new(width, height),
            color,
        }
    }

    pub fn as_vertices(&self) -> [ShapeVertex; 6] {
        let position = self.position.extend(0.0);

        [
            [0.0, 0.0],
            [0.0, self.size.y],
            [self.size.x, self.size.y],
            [0.0, 0.0],
            [self.size.x, 0.0],
            [self.size.x, self.size.y],
        ]
        .map(|offset| {
            let offset = Vec2::from_array(offset).extend(0.0);

            ShapeVertex {
                position: position + offset,
                color: self.color,
            }
        })
    }
}

pub struct ShapeRenderer {
    shader: Program,
    matrix: Option<Mat4>,
}

impl ShapeRenderer {
    pub fn new(display: &WindowDisplay) -> Self {
        Self {
            shader: ShapeShader::program(display),
            matrix: None,
        }
    }

    pub const fn set_matrix(&mut self, matrix: Mat4) {
        self.matrix = Some(matrix);
    }

    pub const fn set_default_matrix(&mut self) {
        self.matrix = None;
    }

    fn draw_shapes(
        &self,
        frame: &mut Frame,
        display: &WindowDisplay,
        vertices: &[ShapeVertex],
        ty: PrimitiveType,
    ) {
        let vertex_buffer = VertexBuffer::new(display, vertices).unwrap();

        let (width, height) = display.get_framebuffer_dimensions();

        let matrix = self.matrix.unwrap_or_else(|| {
            Mat4::orthographic_rh_gl(0., width as f32, height as f32, 0., -1., 1.)
        });

        let uniforms = uniform! {
            matrix: matrix.to_cols_array_2d(),
        };

        frame
            .draw(
                &vertex_buffer,
                NoIndices(ty),
                &self.shader,
                &uniforms,
                &DrawParameters {
                    blend: BLENDING,
                    ..DrawParameters::default()
                },
            )
            .expect("failed to draw!");
    }

    pub fn draw_rects(
        &self,
        frame: &mut Frame,
        display: &WindowDisplay,
        rects: &[Rectangle],
        draw_calls: &mut usize,
        rendered_vertices: &mut usize,
    ) {
        let vertices = rects.iter().fold(Vec::new(), |mut vertices, rect| {
            vertices.extend(rect.as_vertices());

            vertices
        });

        self.draw_shapes(frame, display, &vertices, PrimitiveType::TrianglesList);

        *draw_calls += 1;
        *rendered_vertices += vertices.len();
    }

    pub fn draw_lines(
        &self,
        frame: &mut Frame,
        display: &WindowDisplay,
        lines: &[Line],
        draw_calls: &mut usize,
        rendered_vertices: &mut usize,
    ) {
        let vertices = lines.iter().fold(Vec::new(), |mut vertices, line| {
            vertices.extend(line.as_vertices());

            vertices
        });

        self.draw_shapes(frame, display, &vertices, PrimitiveType::LinesList);

        *draw_calls += 1;
        *rendered_vertices += vertices.len();
    }
}
