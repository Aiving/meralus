use crate::{
    Camera, Game, KeyboardController, game::Aabb, get_movement_direction, get_rotation_directions,
    raycast,
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
    pub is_on_ground: bool,
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
            is_on_ground: false,
        }
    }
}

impl PlayerController {
    pub const MOVE_SPEED: f32 = 4.;
    pub const MOUSE_SENSE: f32 = 0.05;
    pub const LOOK_SPEED: f32 = 0.1;
    pub const GRAVITY: f32 = 9.81 * 1.5;
    pub const AFFECTED_BY_PHYSICS: bool = false;

    pub fn handle_physics(
        &mut self,
        game: &Game,
        keyboard: &KeyboardController,
        camera: &mut Camera,
        delta: f32,
    ) {
        let direction = get_movement_direction(keyboard);

        let (front, right, _) = get_rotation_directions(self.yaw, 0.0);

        let velocity = ((front * direction.z) + (right * direction.x))
            * if keyboard.is_key_pressed(KeyCode::ShiftLeft) && direction.z > 0.0 {
                camera.fov = camera.fov.lerp(65.0_f32.to_radians(), 0.15);

                Self::MOVE_SPEED * 1.5
            } else {
                camera.fov = camera.fov.lerp(55.0_f32.to_radians(), 0.15);

                Self::MOVE_SPEED
            };

        self.velocity.x = velocity.x;
        self.velocity.z = velocity.z;

        if !self.is_on_ground && Self::AFFECTED_BY_PHYSICS {
            self.velocity.y -= Self::GRAVITY * delta;
        }

        if self.is_on_ground && self.velocity.y <= 0.0 && Self::AFFECTED_BY_PHYSICS {
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

        if keyboard.is_key_pressed(KeyCode::Space) && self.is_on_ground && Self::AFFECTED_BY_PHYSICS
        {
            self.velocity.y = 5.0;
        } else if keyboard.is_key_pressed(KeyCode::Space) && !Self::AFFECTED_BY_PHYSICS {
            self.position.y += 0.5;
        }

        if keyboard.is_key_pressed(KeyCode::ControlLeft) && !Self::AFFECTED_BY_PHYSICS {
            self.position.y -= 0.5;
        }

        self.move_and_collide(game, delta);
    }

    pub fn move_and_collide(&mut self, game: &Game, delta: f32) {
        let mut remaining_movement = self.velocity * delta;
        let mut actual_movement = [0.0; 3];

        for axis in 0..3 {
            if remaining_movement[axis] == 0.0 {
                continue;
            }

            let mut test_pos = self.position;

            test_pos[axis] += remaining_movement[axis];

            let test_aabb = Aabb::new(
                test_pos - vec3(0.5, 2.0, 0.5),
                test_pos + vec3(0.5, 0.0, 0.5),
            );

            if game.collides(test_aabb) {
                self.is_on_ground = game.get_colliders(test_pos, test_aabb).bottom.is_some();

                // Try smaller steps for more precision
                let mut step = remaining_movement[axis].abs();
                let direction = remaining_movement[axis].signum();

                #[allow(clippy::while_float)]
                while step > 0.001 {
                    test_pos[axis] = direction.mul_add(step, self.position[axis]);

                    let test_aabb = Aabb::new(
                        test_pos - vec3(0.5, 2.0, 0.5),
                        test_pos + vec3(0.5, 0.0, 0.5),
                    );

                    if !game.collides(test_aabb) {
                        self.position[axis] = test_pos[axis];

                        actual_movement[axis] += direction * step;
                        remaining_movement[axis] -= direction * step;

                        break;
                    }

                    step /= 2.0;
                }
            } else {
                self.position[axis] = test_pos[axis];
                self.is_on_ground = false;

                actual_movement[axis] = remaining_movement[axis];
                remaining_movement[axis] = 0.0;
            }
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
