#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::{
    borrow::Cow,
    num::NonZeroU32,
    time::{Duration, Instant},
};

use glam::{uvec2, vec2, U16Vec3, UVec2, Vec2, Vec3};
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
use meralus_shared::Color;
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
    pub corner: Vec3,
    pub light: u8,
    pub position: u16,
    pub uv: Vec2,
    pub color: Color,
}

impl Vertex {
    const BINDINGS: &[(Cow<'static, str>, usize, i32, AttributeType, bool)] = &[
        (
            Cow::Borrowed("corner"),
            glium::__glium_offset_of!(Vertex, corner),
            -1,
            AttributeType::F32F32F32,
            false,
        ),
        (
            Cow::Borrowed("light"),
            glium::__glium_offset_of!(Vertex, light),
            -1,
            AttributeType::U8,
            false,
        ),
        (
            Cow::Borrowed("position"),
            glium::__glium_offset_of!(Vertex, position),
            -1,
            AttributeType::U16,
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
            Cow::Borrowed("color"),
            glium::__glium_offset_of!(Vertex, color),
            -1,
            AttributeType::U8U8U8U8,
            false,
        ),
    ];

    pub const fn from_vec(
        corner: Vec3,
        position: U16Vec3,
        uv: Vec2,
        light: u8,
        color: Color,
    ) -> Self {
        let position = (position.x << 12) | (position.z << 8) | position.y;

        Self {
            corner,
            light,
            position,
            uv,
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

    /// Runs every 50ms
    fn tick(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: Duration) {}
    /// Runs every 16.66ms
    fn fixed_update(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: f32) {}
    fn update(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: Duration) {}
    fn render(&mut self, event_loop: &ActiveEventLoop, display: &WindowDisplay, delta: f32);
}

pub struct ApplicationWindow<T: State> {
    state: T,
    window: Window,
    display: WindowDisplay,
    last_time: Option<Instant>,
    tick_acceleration: Duration,
    fixed_acceleration: Duration,
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
        let mut window_attrs = Window::default_attributes()
            .with_transparent(false)
            .with_visible(self.visible);

        if let Some(title) = self.title {
            window_attrs.title = title;
        }

        if let Some(size) = self.size {
            window_attrs.inner_size = Some(Size::Physical(PhysicalSize::new(size[0], size[1])));
        }

        let template_builder = ConfigTemplateBuilder::new().with_transparency(true);
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
            tick_acceleration: Duration::ZERO,
            fixed_acceleration: Duration::ZERO,
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

pub const TICK_RATE: Duration = Duration::from_millis(50);
pub const FIXED_FRAMERATE: Duration = Duration::from_secs(1)
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
            window.fixed_acceleration += window.delta;
            window.tick_acceleration += window.delta;

            while window.fixed_acceleration > FIXED_FRAMERATE {
                window.fixed_acceleration -= FIXED_FRAMERATE;

                window.state.fixed_update(
                    event_loop,
                    &window.display,
                    FIXED_FRAMERATE.as_secs_f32(),
                );
            }

            while window.tick_acceleration > TICK_RATE {
                window.tick_acceleration -= TICK_RATE;

                window.state.tick(event_loop, &window.display, TICK_RATE);
            }

            window
                .state
                .update(event_loop, &window.display, window.delta);

            window
                .state
                .render(event_loop, &window.display, window.delta.as_secs_f32());

            window.delta = window
                .last_time
                .map_or_else(|| FIXED_FRAMERATE, |last_time| last_time.elapsed());

            window.last_time.replace(Instant::now());
        });
    }
}
