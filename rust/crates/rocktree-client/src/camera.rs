//! Free-flight camera controller for exploring the Earth.
//!
//! Provides WASD movement with mouse look and altitude-based speed scaling.

use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

/// Plugin for free-flight camera controls.
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_systems(Update, (camera_look, camera_movement));
    }
}

/// Settings for camera movement.
#[derive(Resource)]
pub struct CameraSettings {
    /// Base movement speed in meters per second.
    pub base_speed: f32,
    /// Speed multiplier when boost key is held.
    pub boost_multiplier: f32,
    /// Mouse sensitivity for look rotation.
    pub mouse_sensitivity: f32,
    /// Earth radius in meters (for altitude calculation).
    pub earth_radius: f64,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            base_speed: 1000.0,
            boost_multiplier: 5.0,
            mouse_sensitivity: 0.001,
            earth_radius: 6_371_000.0,
        }
    }
}

/// Marker component for the camera entity that should be controlled.
#[derive(Component)]
pub struct FlightCamera {
    /// Current direction the camera is facing (normalized).
    pub direction: Vec3,
}

impl Default for FlightCamera {
    fn default() -> Self {
        Self {
            direction: Vec3::new(0.219_862, 0.419_329, 0.312_226).normalize(),
        }
    }
}

/// Handle mouse look rotation.
#[allow(clippy::needless_pass_by_value)]
fn camera_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    settings: Res<CameraSettings>,
    mut query: Query<(&mut Transform, &mut FlightCamera)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    for (mut transform, mut camera) in &mut query {
        let yaw = delta.x * settings.mouse_sensitivity;
        let pitch = -delta.y * settings.mouse_sensitivity;

        // Calculate up vector (from Earth center towards camera).
        let up = transform.translation.normalize();

        // Prevent looking straight up or down.
        let overhead = camera.direction.dot(-up);
        let pitch = if (overhead > 0.99 && pitch < 0.0) || (overhead < -0.99 && pitch > 0.0) {
            0.0
        } else {
            pitch
        };

        // Calculate rotation axes.
        let pitch_axis = camera.direction.cross(up).normalize();
        let yaw_axis = camera.direction.cross(pitch_axis).normalize();

        // Apply rotations.
        let yaw_rotation = Quat::from_axis_angle(yaw_axis, yaw);
        let pitch_rotation = Quat::from_axis_angle(pitch_axis, pitch);

        camera.direction = (yaw_rotation * pitch_rotation * camera.direction).normalize();

        // Update transform to look in the new direction.
        transform.look_to(camera.direction, up);
    }
}

/// Handle WASD movement with shift boost.
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
fn camera_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    mut query: Query<(&mut Transform, &FlightCamera)>,
) {
    for (mut transform, camera) in &mut query {
        // Calculate altitude-based speed.
        let position = transform.translation.as_dvec3();
        let altitude = position.length() - settings.earth_radius;
        let altitude = altitude.max(0.0);

        // Speed scales with altitude: faster when high, slower when near ground.
        let speed_factor = ((altitude / 10000.0).max(1.0) + 1.0).powf(1.337) / 6.0;
        let speed_factor = speed_factor.min(2600.0) as f32;

        let mut speed = settings.base_speed * speed_factor;
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            speed *= settings.boost_multiplier;
        }

        // Calculate movement directions.
        let up = transform.translation.normalize();
        let forward = camera.direction;
        let right = forward.cross(up).normalize();

        // Accumulate movement.
        let mut movement = Vec3::ZERO;

        if keyboard.pressed(KeyCode::KeyW) {
            movement += forward;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            movement -= forward;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            movement -= right;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            movement += right;
        }

        if movement != Vec3::ZERO {
            movement = movement.normalize() * speed * time.delta_secs();

            // Check if new position is within reasonable bounds.
            let new_position = transform.translation + movement;
            let new_altitude = new_position.as_dvec3().length() - settings.earth_radius;

            // Prevent going too far from Earth.
            if new_altitude < 10_000_000.0 {
                transform.translation = new_position;
            }
        }
    }
}
