use crate::{Animation, TweenValue};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimationPlayer {
    animations: HashMap<String, Animation>,
    enabled: bool,
}

impl AnimationPlayer {
    pub fn reset(&mut self) {
        for animation in self.animations.values_mut() {
            animation.reset();
        }
    }

    pub const fn is_enabled(&mut self) -> bool {
        self.enabled
    }

    pub const fn enable(&mut self) {
        self.enabled = true;
    }

    pub const fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn advance(&mut self, delta: f32) {
        if self.enabled {
            for animation in self.animations.values_mut() {
                animation.advance(delta);
            }
        }
    }

    pub fn add<T: Into<String>>(&mut self, name: T, animation: Animation) {
        self.animations.insert(name.into(), animation);
    }

    pub fn get_elapsed<T: AsRef<str>>(&self, name: T) -> Option<f32> {
        self.animations
            .get(name.as_ref())
            .map(Animation::get_elapsed)
    }

    pub fn get_duration<T: AsRef<str>>(&self, name: T) -> Option<f32> {
        self.animations
            .get(name.as_ref())
            .map(|animation| animation.duration)
    }

    pub fn get_value<T: AsRef<str>, V: From<TweenValue>>(&self, name: T) -> Option<V> {
        self.animations.get(name.as_ref()).map(Animation::get)
    }

    pub fn animations(&self) -> impl Iterator<Item = (&str, &Animation)> {
        self.animations
            .iter()
            .map(|(name, animation)| (name.as_str(), animation))
    }

    pub fn len(&self) -> usize {
        self.animations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
    }

    pub fn is_finished<T: AsRef<str>>(&self, name: T) -> bool {
        self.enabled
            && self
                .animations
                .get(name.as_ref())
                .is_some_and(Animation::is_finished)
    }
}
