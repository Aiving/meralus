use crate::AsValue;
use glam::{Vec3, Vec4};

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

    pub const fn get_red(&self) -> u8 {
        self.0[0]
    }

    pub const fn get_green(&self) -> u8 {
        self.0[1]
    }

    pub const fn get_blue(&self) -> u8 {
        self.0[2]
    }

    pub const fn get_alpha(&self) -> u8 {
        self.0[3]
    }

    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self([red, green, blue, alpha])
    }

    pub const fn new_f32(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self([
            (255.0 * red) as u8,
            (255.0 * green) as u8,
            (255.0 * blue) as u8,
            (255.0 * alpha) as u8,
        ])
    }

    #[must_use]
    pub const fn with_alpha(mut self, value: f32) -> Self {
        self.0[3] = (255.0 * value) as u8;

        self
    }

    pub const fn from_hsl(hue: f32, saturation: f32, lightness: f32) -> Self {
        let [red, green, blue] = if saturation == 0.0 {
            [lightness, lightness, lightness]
        } else {
            const fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
                if t < 0.0 {
                    t += 1.0;
                }

                if t > 1.0 {
                    t -= 1.0;
                }

                match t {
                    t if t < 1.0 / 6.0 => ((q - p) * 6.0) * t + p,
                    t if t < 1.0 / 2.0 => q,
                    t if t < 2.0 / 3.0 => ((q - p) * (2.0 / 3.0 - t)) * 6.0 + p,
                    _ => p,
                }
            }

            let q = if lightness < 0.5 {
                lightness * (1.0 + saturation)
            } else {
                lightness * -saturation + (lightness + saturation)
            };

            let p = 2.0f32 * lightness - q;

            [
                hue_to_rgb(p, q, (hue / 360.0) + 1.0 / 3.0),
                hue_to_rgb(p, q, hue / 360.0),
                hue_to_rgb(p, q, (hue / 360.0) - 1.0 / 3.0),
            ]
        };

        Self::new_f32(red, green, blue, 1.0)
    }

    #[must_use]
    pub fn multiply_rgb(self, factor: f32) -> Self {
        let value: Vec3 = self.as_value();

        (value * factor).into()
    }
}
