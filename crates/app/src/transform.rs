use glam::{Affine3A, Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::default(),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn set_rotation_x(&mut self, value: f32) {
        self.rotation = Quat::from_axis_angle(Vec3::X, value) * self.rotation;
    }

    pub fn set_rotation_y(&mut self, value: f32) {
        self.rotation = Quat::from_axis_angle(Vec3::Y, value) * self.rotation;
    }

    pub fn set_rotation_z(&mut self, value: f32) {
        self.rotation = Quat::from_axis_angle(Vec3::Z, value) * self.rotation;
    }

    pub fn set_rotation(&mut self, x: f32, y: f32, z: f32) {
        self.set_rotation_x(x);
        self.set_rotation_y(y);
        self.set_rotation_z(z);
    }

    pub const fn set_scale_x(&mut self, value: f32) {
        self.scale.x = value;
    }

    pub const fn set_scale_y(&mut self, value: f32) {
        self.scale.y = value;
    }

    pub const fn set_scale_z(&mut self, value: f32) {
        self.scale.z = value;
    }

    pub const fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        self.set_scale_x(x);
        self.set_scale_y(y);
        self.set_scale_z(z);
    }

    pub fn set_translation_x(&mut self, value: f32) {
        self.translation.x += value;
    }

    pub fn set_translation_y(&mut self, value: f32) {
        self.translation.y += value;
    }

    pub fn set_translation_z(&mut self, value: f32) {
        self.translation.z += value;
    }

    pub fn set_translation(&mut self, x: f32, y: f32, z: f32) {
        self.set_translation_x(x);
        self.set_translation_y(y);
        self.set_translation_z(z);
    }

    #[must_use]
    pub fn affine(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}
