/*
pub struct EGui {
    egui_miniquad: Option<EguiMq>,
    input_subscriber_id: Option<usize>,
}

impl Default for EGui {
    fn default() -> Self {
        Self::new()
    }
}

impl EGui {
    pub const fn new() -> Self {
        Self {
            egui_miniquad: None,
            input_subscriber_id: None,
        }
    }

    pub fn init(&mut self) {
        self.egui_miniquad
            .replace(EguiMq::new(unsafe { get_internal_gl() }.quad_context));

        self.input_subscriber_id
            .replace(macroquad::input::utils::register_input_subscriber());
    }

    pub fn ui<F>(&mut self, f: F)
    where
        F: FnMut(&mut dyn RenderingBackend, &egui::Context),
    {
        let gl = unsafe { get_internal_gl() };

        if let Some(input_subscriber_id) = self.input_subscriber_id {
            repeat_all_miniquad_input(self, input_subscriber_id);
        }

        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.run(gl.quad_context, f);
        }
    }

    pub fn draw(&mut self) {
        let mut gl = unsafe { get_internal_gl() };

        // Ensure that macroquad's shapes are not goint to be lost, and draw them now
        gl.flush();

        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.draw(gl.quad_context);
        }
    }
}

impl EventHandler for EGui {
    fn update(&mut self) {
        todo!()
    }

    fn draw(&mut self) {
        todo!()
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.mouse_motion_event(x, y);
        }
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.mouse_wheel_event(dx, dy);
        }
    }

    fn mouse_button_down_event(&mut self, mb: MouseButton, x: f32, y: f32) {
        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.mouse_button_down_event(mb, x, y);
        }
    }

    fn mouse_button_up_event(&mut self, mb: MouseButton, x: f32, y: f32) {
        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.mouse_button_up_event(mb, x, y);
        }
    }

    fn char_event(&mut self, character: char, _keymods: KeyMods, _repeat: bool) {
        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.char_event(character);
        }
    }

    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, _repeat: bool) {
        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.key_down_event(keycode, keymods);
        }
    }

    fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        if let Some(egui) = self.egui_miniquad.as_mut() {
            egui.key_up_event(keycode, keymods);
        }
    }
}
 */
