mod curves;
mod player;
mod value;

use std::time::Duration;

use meralus_shared::Lerp;

pub use self::{
    curves::{Curve, ICurve},
    player::AnimationPlayer,
    value::TweenValue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepeatMode {
    Once,
    Times(u16),
    Infinite,
}

impl RepeatMode {
    /// Returns `true` if the repeat mode is [`Infinite`].
    ///
    /// [`Infinite`]: RepeatMode::Infinite
    #[must_use]
    pub const fn is_infinite(&self) -> bool {
        matches!(self, Self::Infinite)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RestartBehaviour {
    StartValue,
    EndValue,
}

impl RestartBehaviour {
    /// Returns `true` if the restart behaviour is [`EndValue`].
    ///
    /// [`EndValue`]: RestartBehaviour::EndValue
    #[must_use]
    pub const fn is_end_value(self) -> bool {
        matches!(self, Self::EndValue)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Animation {
    elapsed: f32,
    duration: f32,
    curve: Curve,
    repeat: RepeatMode,
    restart_behaviour: RestartBehaviour,

    origin: TweenValue,
    value: TweenValue,
    destination: TweenValue,
}

impl Animation {
    #[must_use]
    pub fn new<T: Into<TweenValue>>(
        start: T,
        end: T,
        duration: u64,
        curve: Curve,
        repeat: RepeatMode,
    ) -> Self {
        let [origin, destination] = [start.into(), end.into()];

        Self {
            elapsed: 0.0,
            duration: Duration::from_millis(duration).as_secs_f32(),
            curve,
            repeat,
            restart_behaviour: RestartBehaviour::StartValue,
            origin,
            value: origin,
            destination,
        }
    }

    #[must_use]
    pub const fn with_restart_behaviour(mut self, behaviour: RestartBehaviour) -> Self {
        self.restart_behaviour = behaviour;

        self
    }

    pub fn to<T: Into<TweenValue>>(&mut self, value: T) {
        self.origin = self.value;
        self.destination = value.into();
    }

    pub fn get<T: From<TweenValue>>(&self) -> T {
        self.value.into()
    }

    pub const fn is_backwards(&self) -> bool {
        self.restart_behaviour.is_end_value()
    }

    pub const fn get_duration(&self) -> f32 {
        self.duration
    }

    pub const fn get_elapsed(&self) -> f32 {
        match self.repeat {
            RepeatMode::Once | RepeatMode::Infinite => {
                if self.repeat.is_infinite() && self.is_backwards() && self.elapsed >= self.duration
                {
                    self.duration - (self.elapsed.min(self.duration * 2.0) - self.duration)
                } else {
                    self.elapsed.min(self.duration)
                }
            }
            RepeatMode::Times(_) => self.elapsed.min(self.duration) % (self.duration + 1.0),
        }
    }

    pub const fn reset(&mut self) {
        self.elapsed = 0.0;
        self.value = self.origin;
    }

    pub fn advance(&mut self, delta: f32) {
        self.elapsed += delta;

        let t = match (self.repeat, self.restart_behaviour) {
            (RepeatMode::Once, _) | (RepeatMode::Infinite, RestartBehaviour::StartValue) => {
                self.elapsed.min(self.duration) / self.duration
            }
            (RepeatMode::Times(_), _) => {
                (self.elapsed.min(self.duration) % (self.duration + 1.0)) / self.duration
            }
            (RepeatMode::Infinite, RestartBehaviour::EndValue) => {
                if self.elapsed >= self.duration {
                    (self.duration - (self.elapsed.min(self.duration * 2.0) - self.duration))
                        / self.duration
                } else {
                    self.elapsed.min(self.duration) / self.duration
                }
            }
        };

        self.value = self.origin.lerp(&self.destination, self.curve.transform(t));

        if self.repeat.is_infinite()
            && self.elapsed
                >= if self.restart_behaviour.is_end_value() {
                    self.duration * 2.0
                } else {
                    self.duration
                }
        {
            self.elapsed = 0.0;
        }
    }

    pub const fn is_finished(&self) -> bool {
        match self.repeat {
            RepeatMode::Once => self.elapsed >= self.duration,
            RepeatMode::Times(n) => self.elapsed >= (self.duration * n as f32),
            RepeatMode::Infinite => false,
        }
    }
}
