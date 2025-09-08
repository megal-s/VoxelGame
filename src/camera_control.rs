use std::f32::consts::FRAC_PI_2;

use bevy::{
    input::{ButtonInput, keyboard::KeyCode, mouse::AccumulatedMouseMotion},
    math::{EulerRot, Quat, Vec2},
    prelude::*,
    time::Time,
    transform::components::Transform,
};

#[derive(Component)]
#[require(Camera3d)]
pub struct MovableCamera {
    pub speed: f32,
    pub sensitivity: f32,
}

pub struct CameraMovementPlugin;

impl Plugin for CameraMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, control_camera);
    }
}

fn axis(a: bool, b: bool) -> f32 {
    if a && b {
        return 0.;
    }
    if a {
        return 1.;
    }
    if b {
        return -1.;
    }
    0.
}

fn control_camera(
    time: Res<Time>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Single<(&mut Transform, &MovableCamera)>,
) {
    let (mut transform, movable_camera) = camera_query.into_inner();

    let forward = transform.forward().normalize();
    let left = transform.left().normalize();
    let up = transform.up().normalize();
    transform.translation += (forward
        * axis(
            keyboard_input.pressed(KeyCode::KeyW),
            keyboard_input.pressed(KeyCode::KeyS),
        )
        + left
            * axis(
                keyboard_input.pressed(KeyCode::KeyA),
                keyboard_input.pressed(KeyCode::KeyD),
            )
        + up * axis(
            keyboard_input.pressed(KeyCode::Space),
            keyboard_input.pressed(KeyCode::ShiftLeft),
        ))
        * movable_camera.speed
        * time.delta_secs();

    if mouse_motion.delta == Vec2::ZERO {
        return;
    }
    let (mut yaw, mut pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
    yaw += -mouse_motion.delta.x * movable_camera.sensitivity;

    const PITCH_MAX: f32 = FRAC_PI_2 - 0.01;
    pitch = (pitch - mouse_motion.delta.y * movable_camera.sensitivity)
        .clamp(-PITCH_MAX, PITCH_MAX);

    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
}
