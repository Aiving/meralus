use meralus_shared::{Color, Lerp};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum TweenValue {
    Color(Color),
    Float(f32),
}

impl Lerp for TweenValue {
    fn lerp(&self, end: &Self, x: f32) -> Self {
        match (self, end) {
            (Self::Color(a), Self::Color(b)) => Self::Color(a.lerp(b, x)),
            (Self::Float(a), Self::Float(b)) => Self::Float(a.lerp(b, x)),
            _ => unimplemented!(),
        }
    }
}

impl From<Color> for TweenValue {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl From<f32> for TweenValue {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<TweenValue> for Color {
    fn from(value: TweenValue) -> Self {
        match value {
            TweenValue::Color(value) => value,
            TweenValue::Float(_) => unreachable!(),
        }
    }
}

impl From<TweenValue> for f32 {
    fn from(value: TweenValue) -> Self {
        match value {
            TweenValue::Float(value) => value,
            TweenValue::Color(_) => unreachable!(),
        }
    }
}
