#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::{
    borrow::Cow,
    num::NonZeroU32,
    time::{Duration, Instant},
};

use glam::{UVec2, Vec2, Vec3, Vec4, uvec2, vec2, vec3};
pub use glium;
use glium::{Display, vertex::AttributeType};
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder},
    display::GetGlDisplay,
    prelude::{GlDisplay, NotCurrentGlContext},
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::{DeviceEvent, DeviceId, KeyEvent, MouseButton, WindowEvent},
    keyboard::PhysicalKey,
    raw_window_handle::HasWindowHandle,
    window::{CursorGrabMode, Window, WindowId},
};
pub use winit::{
    event_loop::{ActiveEventLoop, EventLoop, EventLoopBuilder},
    keyboard::KeyCode,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub uv: Vec2,
    pub overlay_uv: Vec2,
    pub overlay_color: Color,
    pub have_overlay: u8,
    pub color: Color,
}

pub trait AsValue<T> {
    fn as_value(&self) -> T;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Color type represented as RGBA
pub struct Color([u8; 4]);

impl AsValue<[f32; 4]> for Color {
    fn as_value(&self) -> [f32; 4] {
        [
            f32::from(self.0[0]) / 255.0,
            f32::from(self.0[1]) / 255.0,
            f32::from(self.0[2]) / 255.0,
            f32::from(self.0[3]) / 255.0,
        ]
    }
}

impl AsValue<[f32; 3]> for Color {
    fn as_value(&self) -> [f32; 3] {
        [
            f32::from(self.0[0]) / 255.0,
            f32::from(self.0[1]) / 255.0,
            f32::from(self.0[2]) / 255.0,
        ]
    }
}

impl AsValue<Vec4> for Color {
    fn as_value(&self) -> Vec4 {
        Vec4::from_array(self.as_value())
    }
}

impl AsValue<Vec3> for Color {
    fn as_value(&self) -> Vec3 {
        Vec3::from_array(self.as_value())
    }
}

impl From<Vec4> for Color {
    fn from(value: Vec4) -> Self {
        Self([
            (255.0 * value.x) as u8,
            (255.0 * value.y) as u8,
            (255.0 * value.z) as u8,
            (255.0 * value.w) as u8,
        ])
    }
}

impl From<Vec3> for Color {
    fn from(value: Vec3) -> Self {
        Self([
            (255.0 * value.x) as u8,
            (255.0 * value.y) as u8,
            (255.0 * value.z) as u8,
            255,
        ])
    }
}

impl AsValue<[u8; 4]> for Color {
    fn as_value(&self) -> [u8; 4] {
        self.0
    }
}

impl Color {
    pub const RED: Self = Self([255, 0, 0, 255]);
    pub const GREEN: Self = Self([0, 255, 0, 255]);
    pub const LIGHT_GREEN: Self = Self([122, 250, 129, 255]);
    pub const BLUE: Self = Self([0, 0, 255, 255]);
    pub const YELLOW: Self = Self([255, 255, 0, 255]);
    pub const BROWN: Self = Self([165, 42, 42, 255]);
    pub const PURPLE: Self = Self([128, 0, 128, 255]);
    pub const WHITE: Self = Self([255, 255, 255, 255]);
    pub const BLACK: Self = Self([0, 0, 0, 255]);

    pub fn from_hsl(hue: f32, saturation: f32, lightness: f32) -> Self {
        let [red, green, blue] = if saturation == 0.0 {
            [lightness, lightness, lightness]
        } else {
            fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
                if t < 0.0 {
                    t += 1.0;
                }

                if t > 1.0 {
                    t -= 1.0;
                }

                match t {
                    t if t < 1.0 / 6.0 => ((q - p) * 6.0).mul_add(t, p),
                    t if t < 1.0 / 2.0 => q,
                    t if t < 2.0 / 3.0 => ((q - p) * (2.0 / 3.0 - t)).mul_add(6.0, p),
                    _ => p,
                }
            }

            let q = if lightness < 0.5 {
                lightness * (1.0 + saturation)
            } else {
                lightness.mul_add(-saturation, lightness + saturation)
            };

            let p = 2.0f32.mul_add(lightness, -q);

            [
                hue_to_rgb(p, q, hue + 1.0 / 3.0),
                hue_to_rgb(p, q, hue),
                hue_to_rgb(p, q, hue - 1.0 / 3.0),
            ]
        };

        Self::from(vec3(red, green, blue))
    }

    #[must_use]
    pub fn multiply_rgb(self, factor: f32) -> Self {
        let value: Vec3 = self.as_value();

        (value * factor).into()
    }
}

impl Vertex {
    const BINDINGS: &[(Cow<'static, str>, usize, i32, AttributeType, bool)] = &[
        (
            Cow::Borrowed("position"),
            glium::__glium_offset_of!(Vertex, position),
            -1,
            AttributeType::F32F32F32,
            false,
        ),
        (
            Cow::Borrowed("uv"),
            glium::__glium_offset_of!(Vertex, uv),
            -1,
            AttributeType::F32F32,
            false,
        ),
        (
            Cow::Borrowed("overlay_uv"),
            glium::__glium_offset_of!(Vertex, overlay_uv),
            -1,
            AttributeType::F32F32,
            false,
        ),
        (
            Cow::Borrowed("overlay_color"),
            glium::__glium_offset_of!(Vertex, overlay_color),
            -1,
            AttributeType::U8U8U8U8,
            false,
        ),
        (
            Cow::Borrowed("have_overlay"),
            glium::__glium_offset_of!(Vertex, have_overlay),
            -1,
            AttributeType::U8,
            false,
        ),
        (
            Cow::Borrowed("color"),
            glium::__glium_offset_of!(Vertex, color),
            -1,
            AttributeType::U8U8U8U8,
            false,
        ),
    ];

    pub const fn from_vec(
        position: Vec3,
        uv: Vec2,
        color: Color,
        overlay_uv: Option<Vec2>,
        overlay_color: Option<Color>,
        have_overlay: bool,
    ) -> Self {
        Self {
            position,
            uv,
            overlay_uv: if let Some(overlay_uv) = overlay_uv {
                overlay_uv
            } else {
                Vec2::ZERO
            },
            overlay_color: if let Some(overlay_color) = overlay_color {
                overlay_color
            } else {
                Color::WHITE
            },
            have_overlay: if have_overlay { 1 } else { 0 },
            color,
        }
    }
}

impl glium::Vertex for Vertex {
    fn build_bindings() -> glium::VertexFormat {
        Self::BINDINGS
    }
}

pub type WindowDisplay = Display<WindowSurface>;

#[allow(unused)]
pub trait State {
    fn new(display: &WindowDisplay) -> Self;

    fn handle_window_resize(
        &mut self,
        event_loop: &ActiveEventLoop,
        size: UVec2,
        scale_factor: f64,
    ) {
    }
    fn handle_keyboard_modifiers(&mut self, event_loop: &ActiveEventLoop, position: [f64; 2]) {}
    fn handle_keyboard_input(&mut self, event_loop: &ActiveEventLoop, event: KeyEvent) {}
    fn handle_mouse_motion(&mut self, event_loop: &ActiveEventLoop, position: Vec2) {}
    fn handle_mouse_button(
        &mut self,
        event_loop: &ActiveEventLoop,
        button: MouseButton,
        is_pressed: bool,
    ) {
    }

    fn update(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: f32) {}
    fn fixed_update(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: f32) {}
    fn render(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay);
}

pub struct ApplicationWindow<T: State> {
    state: T,
    window: Window,
    display: WindowDisplay,
    last_time: Option<Instant>,
    acceleration: Duration,
    delta: Duration,
    cursor_grab: bool,
}

pub struct Application<T: State> {
    window: Option<ApplicationWindow<T>>,
}

impl<T: State> Default for Application<T> {
    fn default() -> Self {
        Self { window: None }
    }
}

pub struct ApplicationWindowBuilder {
    title: Option<String>,
    visible: bool,
    size: Option<[u32; 2]>,
}

impl Default for ApplicationWindowBuilder {
    fn default() -> Self {
        Self {
            title: None,
            visible: true,
            size: None,
        }
    }
}

impl ApplicationWindowBuilder {
    #[must_use]
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());

        self
    }

    #[must_use]
    pub const fn with_visibility(mut self, visible: bool) -> Self {
        self.visible = visible;

        self
    }

    #[must_use]
    pub const fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = Some([width, height]);

        self
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn build<T: State>(self, event_loop: &ActiveEventLoop) -> ApplicationWindow<T> {
        let mut window_attrs = Window::default_attributes().with_visible(self.visible);

        if let Some(title) = self.title {
            window_attrs.title = title;
        }

        if let Some(size) = self.size {
            window_attrs.inner_size = Some(Size::Physical(PhysicalSize::new(size[0], size[1])));
        }

        let template_builder = ConfigTemplateBuilder::new();
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attrs));

        let (window, gl_config) = display_builder
            .build(event_loop, template_builder, |mut configs| {
                configs.next().expect("failed to retrieve configuration")
            })
            .expect("failed to build display");

        let window = window.expect("failed to get window");

        let window_handle = window.window_handle().expect("failed to get window handle");
        let context_attrs = ContextAttributesBuilder::new().build(Some(window_handle.into()));
        let fallback_context_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(Some(window_handle.into()));

        let gl_context = unsafe {
            gl_config
                .display()
                .create_context(&gl_config, &context_attrs)
                .unwrap_or_else(|_| {
                    gl_config
                        .display()
                        .create_context(&gl_config, &fallback_context_attrs)
                        .expect("failed to create context")
                })
        };

        let (width, height): (u32, u32) = window.inner_size().into();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.into(),
            NonZeroU32::new(width).expect("failed to create window width"),
            NonZeroU32::new(height).expect("failed to create window height"),
        );

        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .expect("failed to create surface")
        };

        let current_context = gl_context
            .make_current(&surface)
            .expect("failed to obtain opengl context");

        let display = Display::from_context_surface(current_context, surface)
            .expect("failed to create display from context and surface");

        ApplicationWindow {
            state: T::new(&display),
            window,
            display,
            last_time: None,
            acceleration: Duration::ZERO,
            delta: FIXED_FRAMERATE,
            cursor_grab: false,
        }
    }
}

trait InspectMut<T> {
    fn inspect_mut<F: FnOnce(&mut T)>(&mut self, func: F);
}

impl<T> InspectMut<T> for Option<T> {
    fn inspect_mut<F: FnOnce(&mut T)>(&mut self, func: F) {
        if let Some(data) = self {
            func(data);
        }
    }
}

const FIXED_FRAMERATE: Duration = Duration::from_secs(1)
    .checked_div(60)
    .expect("failed to calculate fixed framerate somehow");

impl<T: State> ApplicationHandler for Application<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut window = ApplicationWindowBuilder::default().build(event_loop);

        window
            .window
            .set_cursor_grab(CursorGrabMode::Confined)
            .expect("failed to grab cursor");

        window.window.set_cursor_visible(false);

        window.cursor_grab = true;

        self.window.replace(window);
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.window.take();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(physical_size) => self.window.inspect_mut(move |window| {
                window.display.resize(physical_size.into());

                window.state.handle_window_resize(
                    event_loop,
                    uvec2(physical_size.width, physical_size.height),
                    window.window.scale_factor(),
                );
            }),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::Tab && event.state.is_pressed() {
                        self.window.inspect_mut(|window| {
                            if window.cursor_grab {
                                window
                                    .window
                                    .set_cursor_grab(CursorGrabMode::None)
                                    .expect("failed to grab cursor");

                                window.window.set_cursor_visible(true);
                            } else {
                                window
                                    .window
                                    .set_cursor_grab(CursorGrabMode::Confined)
                                    .expect("failed to grab cursor");

                                window.window.set_cursor_visible(false);
                            }

                            window.cursor_grab = !window.cursor_grab;
                        });
                    } else {
                        self.window.inspect_mut(move |window| {
                            window.state.handle_keyboard_input(event_loop, event);
                        });
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.window.inspect_mut(|window| {
                    window
                        .state
                        .handle_mouse_button(event_loop, button, state.is_pressed());
                });
            }
            _ => {}
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.window.inspect_mut(|window| {
                if window.cursor_grab {
                    window
                        .state
                        .handle_mouse_motion(event_loop, vec2(delta.0 as f32, delta.1 as f32));
                }
            });
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.window.inspect_mut(|window| {
            window.acceleration += window.delta;

            while window.acceleration > FIXED_FRAMERATE {
                window.acceleration -= FIXED_FRAMERATE;

                window.state.fixed_update(
                    event_loop,
                    &window.display,
                    FIXED_FRAMERATE.as_secs_f32(),
                );
            }

            window
                .state
                .update(event_loop, &window.display, window.delta.as_secs_f32());

            window.state.render(event_loop, &window.display);

            window.delta = window
                .last_time
                .map_or_else(|| FIXED_FRAMERATE, |last_time| last_time.elapsed());

            window.last_time.replace(Instant::now());
        });
    }
}
