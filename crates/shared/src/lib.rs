#![allow(
    clippy::missing_errors_doc,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

mod color;
mod lerp;
#[cfg(feature = "network")] mod network;

#[cfg(feature = "network")]
pub use self::network::{Client, IncomingPacket, OutgoingPacket, Player, ServerConnection};
pub use self::{color::Color, lerp::Lerp};

pub type Size2D = glamour::Size2;
pub type Point2D = glamour::Vector2;
pub type Rect2D = glamour::Rect;

pub trait AsValue<T> {
    fn as_value(&self) -> T;
}

pub trait FromValue<T> {
    fn from_value(value: &T) -> Self;
}

impl<A, T> FromValue<T> for A
where
    T: AsValue<A>,
{
    fn from_value(value: &T) -> Self {
        value.as_value()
    }
}
