use glam::{Mat4, Vec3, vec3};

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub const fn default() -> Self {
        Self {
            position: vec3(0., -10., 0.),
            target: vec3(0., 0., 0.),
            aspect_ratio: 1024.0 / 768.0,
            up: vec3(0., 0., 1.),
            fov: 55.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 10000.0,
        }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov, self.aspect_ratio, self.z_near, self.z_far)
            * Mat4::look_at_rh(self.position, self.target, self.up)
    }
}
