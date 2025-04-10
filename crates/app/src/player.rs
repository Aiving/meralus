use crate::{Game, get_movement_direction, get_rotation_directions};
use macroquad::{
    input::{KeyCode, is_key_down, is_key_pressed},
    math::{Vec2, Vec3, vec3},
};

pub struct PlayerController {
    pub position: Vec3,
    // START CAMERA
    pub yaw: f32,
    pub pitch: f32,
    pub front: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    // END CAMERA
    pub velocity: Vec3,
}

impl Default for PlayerController {
    fn default() -> Self {
        let yaw: f32 = 0.0;
        let pitch: f32 = 0.0;

        let front = vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalize();

        let right = front.cross(Vec3::Y).normalize();
        let up = right.cross(front).normalize();

        Self {
            position: Vec3::Y,
            yaw,
            pitch,
            front,
            right,
            up,
            velocity: Vec3::ZERO,
        }
    }
}

impl PlayerController {
    pub const MOVE_SPEED: f32 = 4.;
    pub const LOOK_SPEED: f32 = 0.1;
    pub const GRAVITY: f32 = 9.81;

    #[must_use]
    pub fn is_on_ground(&self, game: &Game) -> bool {
        game.find_block(self.position - vec3(0.0, 2.0, 0.0))
            .is_some()
    }

    pub fn handle_physics(&mut self, game: &Game, delta: f32) {
        let direction = get_movement_direction();

        let (front, right, _) = get_rotation_directions(self.yaw, 0.0);

        let velocity = ((front * direction.z) + (right * direction.x))
            * if is_key_down(KeyCode::LeftControl) && direction.z > 0.0 {
                Self::MOVE_SPEED * 1.5
            } else {
                Self::MOVE_SPEED
            };

        self.velocity.x = velocity.x;
        self.velocity.z = velocity.z;

        if !self.is_on_ground(game) {
            self.velocity.y -= Self::GRAVITY * delta;
        }

        if self.is_on_ground(game) && self.velocity.y <= 0.0 {
            self.velocity.y = 0.0;
        }

        // if is_key_pressed(KeyCode::Space)
        // /* && self.is_on_ground(game) */
        // {
        //     if is_key_down(KeyCode::LeftShift) {
        //         self.velocity.y -= 5.0;
        //     } else {
        //         self.velocity.y = 5.0;
        //     }
        // }

        if is_key_pressed(KeyCode::Space) && self.is_on_ground(game) {
            self.velocity.y = 5.0;
        }

        self.move_and_collide(game, delta);
    }

    pub fn move_and_collide(&mut self, game: &Game, delta: f32) {
        let position = self.position + (self.velocity * delta);

        if game.find_chunk(position).is_some() {
            let have_under_block = game.find_block(position - Vec3::Y).is_some();

            if !have_under_block {
                self.position.y = position.y;
            }

            self.position.x = position.x;
            self.position.z = position.z;
        }
    }

    pub fn handle_mouse(&mut self, mouse_delta: Vec2, delta: f32) {
        self.yaw += mouse_delta.x * delta * Self::LOOK_SPEED;
        self.pitch += mouse_delta.y * delta * -Self::LOOK_SPEED;

        self.pitch = if self.pitch > 1.5 { 1.5 } else { self.pitch };
        self.pitch = if self.pitch < -1.5 { -1.5 } else { self.pitch };

        self.front = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize();

        self.right = self.front.cross(Vec3::Y).normalize();
        self.up = self.right.cross(self.front).normalize();
    }
}
