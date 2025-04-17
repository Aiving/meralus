use crate::{
    Camera3D, Game, KeyboardController, get_movement_direction, get_rotation_directions, raycast,
};
use glam::{FloatExt, Vec2, Vec3, vec3};
use meralus_engine::KeyCode;

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
    pub const MOUSE_SENSE: f32 = 0.05;
    pub const LOOK_SPEED: f32 = 0.1;
    pub const GRAVITY: f32 = 9.81 * 1.5;
    pub const AFFECTED_BY_PHYSICS: bool = true;

    #[must_use]
    pub fn is_on_ground(&self, game: &Game) -> bool {
        // raycast(game, self.position.as_ivec3(), Vec3::NEG_Y, 2.0).is_some()
        game.block_exists(self.position.mul_add(Vec3::ONE, Vec3::NEG_Y * 2.0))
    }

    pub fn handle_physics(
        &mut self,
        game: &Game,
        keyboard: &KeyboardController,
        camera: &mut Camera3D,
        delta: f32,
    ) {
        let direction = get_movement_direction(keyboard);

        let (front, right, _) = get_rotation_directions(self.yaw, 0.0);

        let velocity = ((front * direction.z) + (right * direction.x))
            * if keyboard.is_key_pressed(KeyCode::ShiftLeft) && direction.z > 0.0 {
                camera.fovy = camera.fovy.lerp(65.0_f32.to_radians(), 0.15);

                Self::MOVE_SPEED * 1.5
            } else {
                camera.fovy = camera.fovy.lerp(55.0_f32.to_radians(), 0.15);

                Self::MOVE_SPEED
            };

        self.velocity.x = velocity.x;
        self.velocity.z = velocity.z;

        if !self.is_on_ground(game) && Self::AFFECTED_BY_PHYSICS {
            self.velocity.y -= Self::GRAVITY * delta;
        }

        if self.is_on_ground(game) && self.velocity.y <= 0.0 && Self::AFFECTED_BY_PHYSICS {
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

        if keyboard.is_key_pressed(KeyCode::Space)
            && (self.is_on_ground(game) || !Self::AFFECTED_BY_PHYSICS)
        {
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

    pub fn handle_mouse(&mut self, looking_at: &mut Option<Vec3>, game: &Game, mouse_delta: Vec2) {
        self.yaw += mouse_delta.x * Self::MOUSE_SENSE * Self::LOOK_SPEED;
        self.pitch += mouse_delta.y * Self::MOUSE_SENSE * -Self::LOOK_SPEED;

        self.pitch = if self.pitch > 1.5 { 1.5 } else { self.pitch };
        self.pitch = if self.pitch < -1.5 { -1.5 } else { self.pitch };

        self.front = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize();

        *looking_at = raycast(game, self.position.floor().as_ivec3(), self.front, 30.0);

        self.right = self.front.cross(Vec3::Y).normalize();
        self.up = self.right.cross(self.front).normalize();
    }
}
