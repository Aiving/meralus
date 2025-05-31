use glam::Mat4;
use glium::{Frame, Rect};
use meralus_engine::WindowDisplay;
use meralus_shared::{Color, Point2D, Rect2D, Size2D};

use crate::{GameLoop, renderers::Rectangle};

struct Text {
    position: Point2D,
    font: String,
    data: String,
    size: f32,
    color: Color,
    clip: Option<Rect2D>,
    matrix: Option<Mat4>,
}

pub struct UiContext<'a> {
    window_size: Size2D,
    bounds: Rect2D,
    pub game_loop: &'a mut GameLoop,
    display: &'a WindowDisplay,
    frame: &'a mut Frame,
    rectangles: Vec<Rectangle>,
    texts: Vec<Text>,
    clip: Option<Rect2D>,
    matrix: Option<Mat4>,
}

impl<'a> UiContext<'a> {
    pub fn new(
        game_loop: &'a mut GameLoop,
        display: &'a WindowDisplay,
        frame: &'a mut Frame,
    ) -> Self {
        let (width, height) = display.get_framebuffer_dimensions();

        Self {
            window_size: Size2D::new(width as f32, height as f32),
            bounds: Rect2D::new(Point2D::ZERO, Size2D::new(width as f32, height as f32)),
            game_loop,
            display,
            frame,
            rectangles: Vec::new(),
            texts: Vec::new(),
            clip: None,
            matrix: None,
        }
    }

    pub fn measure_text<F: AsRef<str>, T: AsRef<str>>(
        &mut self,
        font: F,
        text: T,
        size: f32,
    ) -> Option<Size2D> {
        self.game_loop.text_renderer.measure(font, text, size)
    }

    pub fn draw_text<F: Into<String>, T: Into<String>>(
        &mut self,
        position: Point2D,
        font: F,
        text: T,
        size: f32,
        color: Color,
    ) {
        self.texts.push(Text {
            position,
            font: font.into(),
            data: text.into(),
            size,
            color,
            clip: self.clip,
            matrix: self.matrix,
        });
    }

    pub const fn add_transform(&mut self, transform: Mat4) {
        self.matrix.replace(transform);
    }

    pub const fn remove_transform(&mut self) {
        self.matrix.take();
    }

    pub fn draw_rect(&mut self, position: Point2D, size: Size2D, color: Color) {
        self.rectangles.push(
            Rectangle::new(position.x, position.y, size.width, size.height, color)
                .with_matrix(self.matrix),
        );
    }

    pub fn finish(self) {
        self.game_loop.shape_renderer.draw_rects(
            self.frame,
            self.display,
            &self.rectangles,
            &mut self.game_loop.debugging.draw_calls,
            &mut self.game_loop.debugging.vertices,
        );

        for text in self.texts {
            self.game_loop.text_renderer.render(
                self.frame,
                &(self.game_loop.window_matrix * text.matrix.unwrap_or_default()),
                text.position,
                text.font,
                text.data,
                text.size,
                text.color,
                text.clip.map(|area| Rect {
                    left: area.origin.x.floor() as u32,
                    bottom: (self.window_size.height - area.origin.y - area.size.height).floor()
                        as u32,
                    width: area.size.width.floor() as u32,
                    height: area.size.height.floor() as u32,
                }),
                &mut self.game_loop.debugging.draw_calls,
            );
        }
    }

    pub fn ui<F: FnOnce(&mut UiContext, Rect2D)>(&mut self, func: F) {
        func(self, self.bounds);
    }

    pub fn fill(&mut self, color: Color) {
        self.draw_rect(self.bounds.origin, self.bounds.size, color);
    }

    pub fn clipped<F: FnOnce(&mut UiContext, Rect2D)>(&mut self, bounds: Rect2D, func: F) {
        self.clip.replace(bounds);

        func(self, self.bounds);

        self.clip.take();
    }

    pub fn bounds<F: FnOnce(&mut UiContext, Rect2D)>(&mut self, bounds: Rect2D, func: F) {
        let temp = self.bounds;

        self.bounds = bounds;

        func(self, self.bounds);

        self.bounds = temp;
    }

    pub fn padding<F: FnOnce(&mut UiContext, Rect2D)>(&mut self, value: f32, func: F) {
        self.bounds.origin += Point2D::ONE.to_vector() * value;
        self.bounds.size -= Size2D::ONE * value * 2.0;
        self.bounds.size = self.bounds.size.max(Size2D::ZERO);

        func(self, self.bounds);

        self.bounds.origin -= Point2D::ONE.to_vector() * value;
        self.bounds.size += Size2D::ONE * value * 2.0;
    }
}
