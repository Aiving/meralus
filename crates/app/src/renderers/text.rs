use super::Shader;
use crate::{BLENDING, Point2D, Size2D, impl_vertex, loaders::TextureAtlas};
use fontdue::{
    Font, FontSettings,
    layout::{CoordinateSystem, GlyphRasterConfig, Layout, TextStyle},
};
use glam::{Mat4, Vec2, Vec3, vec2, vec3};
use image::ImageBuffer;
use meralus_engine::{
    WindowDisplay,
    glium::{
        DrawParameters, Frame, Program, Rect, Surface, VertexBuffer,
        index::{NoIndices, PrimitiveType},
        uniform,
        uniforms::MagnifySamplerFilter,
        vertex::BufferCreationError,
    },
};
use meralus_shared::{Color, FromValue};
use std::{borrow::Borrow, collections::HashMap};

pub const FONT: &[u8] = include_bytes!("../../resources/PixeloidSans.ttf");
pub const FONT_BOLD: &[u8] = include_bytes!("../../resources/PixeloidSans-Bold.ttf");

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextVertex {
    pub position: Vec3,
    pub character: Vec2,
}

impl_vertex! {
    TextVertex {
        position: [f32; 3],
        character: [f32; 2]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextDataVertex {
    pub screen_position: Point2D,
    pub offset: Vec2,
    pub size: Vec2,
}

impl_vertex! {
    TextDataVertex {
        screen_position: [f32; 2],
        offset: [f32; 2],
        size: [f32; 2]
    }
}

impl TextDataVertex {
    pub const fn from_vec(screen_position: Point2D, offset: Vec2, size: Vec2) -> Self {
        Self {
            screen_position,
            offset,
            size,
        }
    }
}

struct TextShader;

impl Shader for TextShader {
    const VERTEX: &str = include_str!("../../resources/shaders/text.vs");
    const FRAGMENT: &str = include_str!("../../resources/shaders/text.fs");
}

pub struct FontInfo {
    pub font: Font,
    pub atlas: TextureAtlas<GlyphRasterConfig>,
}

impl Borrow<Font> for FontInfo {
    fn borrow(&self) -> &Font {
        &self.font
    }
}

pub struct TextRenderer {
    character: VertexBuffer<TextVertex>,
    character_offset: VertexBuffer<TextDataVertex>,
    font_name_map: HashMap<String, usize>,
    fonts: Vec<FontInfo>,
    layout: Layout,
    shader: Program,
}

impl TextRenderer {
    pub fn new(
        display: &WindowDisplay,
        character_limit: usize,
    ) -> Result<Self, BufferCreationError> {
        let character = VertexBuffer::new(
            display,
            &[
                TextVertex {
                    position: vec3(0.0, 1.0, 0.0),
                    character: vec2(0.0, 0.0),
                },
                TextVertex {
                    position: vec3(0.0, 0.0, 0.0),
                    character: vec2(0.0, 1.0),
                },
                TextVertex {
                    position: vec3(1.0, 1.0, 0.0),
                    character: vec2(1.0, 0.0),
                },
                TextVertex {
                    position: vec3(1.0, 0.0, 0.0),
                    character: vec2(1.0, 1.0),
                },
            ],
        )?;

        let character_offset = VertexBuffer::dynamic(
            display,
            &(0..character_limit)
                .map(|_| TextDataVertex::from_vec(Point2D::ZERO, Vec2::ZERO, Vec2::ZERO))
                .collect::<Vec<_>>(),
        )?;

        Ok(Self {
            layout: Layout::new(CoordinateSystem::PositiveYDown),
            character,
            character_offset,
            font_name_map: HashMap::new(),
            fonts: Vec::new(),
            shader: TextShader::program(display),
        })
    }

    pub fn add_font<T: Into<String>>(&mut self, display: &WindowDisplay, name: T, data: &[u8]) {
        if let Ok(font) = Font::from_bytes(data, FontSettings::default()) {
            self.font_name_map.insert(name.into(), self.fonts.len());

            self.fonts.push(FontInfo {
                font,
                atlas: TextureAtlas::new(display, 4096),
            });
        }
    }

    pub fn measure<F: AsRef<str>, T: AsRef<str>>(
        &mut self,
        font: F,
        text: T,
        size: f32,
    ) -> Option<Size2D> {
        self.font_name_map
            .get(font.as_ref())
            .copied()
            .map(|font_index| {
                let text = text.as_ref();

                self.layout.clear();
                self.layout
                    .append(&self.fonts, &TextStyle::new(text, size, font_index));

                self.layout
                    .glyphs()
                    .iter()
                    .fold(Size2D::ZERO, |mut metrics, glyph| {
                        metrics.width = metrics.width.max(glyph.x + glyph.width as f32);
                        metrics.height = metrics.height.max(glyph.y + glyph.height as f32);

                        metrics
                    })
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<F: AsRef<str>, T: AsRef<str>>(
        &mut self,
        frame: &mut Frame,
        matrix: &Mat4,
        position: Point2D,
        font: F,
        text: T,
        size: f32,
        color: Color,
        clip_area: Option<Rect>,
        draw_calls: &mut usize,
    ) {
        if let Some(font_index) = self.font_name_map.get(font.as_ref()).copied() {
            let text = text.as_ref();

            self.layout.clear();
            self.layout
                .append(&self.fonts, &TextStyle::new(text, size, font_index));

            let glyphs = self.layout.glyphs();
            let font_info = &mut self.fonts[font_index];

            for (i, vertex) in self.character_offset.map().iter_mut().enumerate() {
                if let Some(glyph) = glyphs.get(i) {
                    if glyph.width == 0 || glyph.height == 0 {
                        vertex.screen_position = Point2D::ZERO;
                        vertex.offset = Vec2::ZERO;
                        vertex.size = Vec2::ZERO;

                        continue;
                    }

                    let (offset, size) = if let Some(rect) = font_info.atlas.get_rect(&glyph.key) {
                        rect
                    } else {
                        let (metrics, bitmap) = font_info.font.rasterize(glyph.parent, size);

                        let mut image =
                            ImageBuffer::new(metrics.width as u32, metrics.height as u32);

                        for (i, pixel) in image.pixels_mut().enumerate() {
                            let alpha = bitmap[i];

                            *pixel = image::Rgba([255, 255, 255, alpha]);
                        }

                        font_info.atlas.append(glyph.key, image)
                    };

                    vertex.screen_position = position + Point2D::new(glyph.x, glyph.y);
                    vertex.offset = offset;
                    vertex.size = size;
                } else {
                    vertex.screen_position = Point2D::ZERO;
                    vertex.offset = Vec2::ZERO;
                    vertex.size = Vec2::ZERO;
                }
            }

            let uniforms = uniform! {
                matrix: matrix.to_cols_array_2d(),
                font: font_info
                    .atlas
                    .get_texture()
                    .sampled()
                    .magnify_filter(MagnifySamplerFilter::Nearest),
                text_color: <[f32; 4]>::from_value(&color),
            };

            frame
                .draw(
                    (
                        &self.character,
                        self.character_offset.per_instance().unwrap(),
                    ),
                    NoIndices(PrimitiveType::TriangleStrip),
                    &self.shader,
                    &uniforms,
                    &DrawParameters {
                        blend: BLENDING,
                        scissor: clip_area,
                        ..Default::default()
                    },
                )
                .expect("failed to draw!");

            *draw_calls += 1;
        }
    }
}
