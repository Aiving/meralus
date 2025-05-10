use crate::{GameLoop, renderers::Rectangle};
use meralus_engine::{
    WindowDisplay,
    glium::{Frame, Rect},
};
use meralus_shared::{Color, Point2D, Rect2D, Size2D};

struct Text {
    position: Point2D,
    font: String,
    data: String,
    size: f32,
    color: Color,
    clip: Option<Rect2D>,
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
        });
    }

    pub fn draw_rect(&mut self, position: Point2D, size: Size2D, color: Color) {
        self.rectangles.push(Rectangle::new(
            position.x,
            position.y,
            size.width,
            size.height,
            color,
        ));
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
                &self.game_loop.window_matrix,
                text.position,
                text.font,
                text.data,
                text.size,
                text.color,
                text.clip.map(|area| Rect {
                    left: area.origin.x.floor() as u32,
                    bottom: (area.origin.y).floor() as u32,
                    width: area.size.width.floor() as u32,
                    height: self.window_size.height.floor() as u32,
                }),
                &mut self.game_loop.debugging.draw_calls,
            );
        }
    }

    pub fn ui<F: FnOnce(&mut UiContext, Rect2D)>(&mut self, func: F) {
        func(self, self.bounds);
    }

    pub fn fill(&mut self, color: Color) {
        self.draw_rect(self.bounds.origin.into(), self.bounds.size, color);
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
        self.bounds.origin += Point2D::ONE * value;
        self.bounds.size -= Size2D::ONE * value * 2.0;

        func(self, self.bounds);

        self.bounds.origin -= Point2D::ONE * value;
        self.bounds.size += Size2D::ONE * value * 2.0;
    }
}
