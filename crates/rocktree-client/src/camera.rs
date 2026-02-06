//! Free-flight camera controller for exploring the Earth.
//!
//! Provides WASD movement with mouse look and altitude-based speed scaling.
//! Works with the floating origin system for high-precision positioning.

use bevy::ecs::message::MessageReader;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_egui::EguiContexts;
use bevy_egui::input::egui_wants_any_keyboard_input;
use glam::DVec3;

use crate::floating_origin::{FloatingOrigin, FloatingOriginCamera};

/// Minimum base speed in meters per second.
pub const MIN_SPEED: f32 = 10.0;
/// Maximum base speed in meters per second.
pub const MAX_SPEED: f32 = 25_000.0;

/// Plugin for free-flight camera controls.
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_systems(Startup, grab_cursor)
            .add_systems(
                Update,
                (
                    cursor_grab_system,
                    adjust_speed_with_scroll.run_if(cursor_is_grabbed),
                    camera_look.run_if(cursor_is_grabbed),
                    camera_movement.run_if(not(egui_wants_any_keyboard_input)),
                    sync_floating_origin,
                )
                    .chain(),
            );
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

/// Grab the cursor on startup.
fn grab_cursor(
    mut cursor: Single<&mut CursorOptions>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    set_cursor_grab(&mut cursor, &mut window, true);
}

/// Set cursor grab state, centering the cursor when grabbing.
fn set_cursor_grab(cursor: &mut CursorOptions, window: &mut Window, grabbed: bool) {
    if grabbed {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
        // Center the cursor in the window.
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
    } else {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Check if cursor is currently grabbed.
#[allow(clippy::needless_pass_by_value)]
fn cursor_is_grabbed(cursor: Single<&CursorOptions>) -> bool {
    cursor.grab_mode == CursorGrabMode::Locked
}

/// Handle cursor grab/ungrab with ESC and left-click.
#[allow(clippy::needless_pass_by_value)]
fn cursor_grab_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cursor: Single<&mut CursorOptions>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut contexts: EguiContexts,
) {
    let is_grabbed = cursor.grab_mode == CursorGrabMode::Locked;

    // ESC to release cursor.
    if keyboard.just_pressed(KeyCode::Escape) && is_grabbed {
        set_cursor_grab(&mut cursor, &mut window, false);
        return;
    }

    // Left-click to grab cursor (when not grabbed and not clicking on UI).
    if mouse.just_pressed(MouseButton::Left) && !is_grabbed {
        let egui_wants_pointer = contexts
            .ctx_mut()
            .ok()
            .is_some_and(|ctx| ctx.is_pointer_over_area());

        if !egui_wants_pointer {
            set_cursor_grab(&mut cursor, &mut window, true);
        }
    }
}

/// Adjust speed with mouse scroll wheel.
#[allow(clippy::needless_pass_by_value)]
fn adjust_speed_with_scroll(
    mut scroll_events: MessageReader<MouseWheel>,
    mut settings: ResMut<CameraSettings>,
) {
    for event in scroll_events.read() {
        // Adjust speed logarithmically for smooth scaling.
        let scroll = event.y;
        if scroll != 0.0 {
            let factor = 1.1_f32.powf(scroll);
            settings.base_speed = (settings.base_speed * factor).clamp(MIN_SPEED, MAX_SPEED);
        }
    }
}

/// Handle mouse look rotation.
#[allow(clippy::needless_pass_by_value)]
fn camera_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    settings: Res<CameraSettings>,
    mut query: Query<(&FloatingOriginCamera, &mut Transform, &mut FlightCamera)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    for (origin_camera, mut transform, mut camera) in &mut query {
        let yaw = delta.x * settings.mouse_sensitivity;
        let pitch = -delta.y * settings.mouse_sensitivity;

        // Calculate up vector (from Earth center towards camera) using high-precision position.
        let up = origin_camera.position.normalize().as_vec3();

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
        // Camera stays at origin; only rotation changes.
        transform.look_to(camera.direction, up);
    }
}

/// Handle WASD + Space/Ctrl movement with shift boost.
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
fn camera_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    mut query: Query<(&mut FloatingOriginCamera, &FlightCamera)>,
) {
    for (mut origin_camera, camera) in &mut query {
        // Calculate altitude-based speed using high-precision position.
        let altitude = origin_camera.position.length() - settings.earth_radius;
        let altitude = altitude.max(0.0);

        // Speed scales with altitude: faster when high, slower when near ground.
        let speed_factor = ((altitude / 10000.0).max(1.0) + 1.0).powf(1.337) / 6.0;
        let speed_factor = speed_factor.min(2600.0) as f32;

        let mut speed = settings.base_speed * speed_factor;
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            speed *= settings.boost_multiplier;
        }

        // Calculate movement directions using high-precision up vector.
        let up = origin_camera.position.normalize().as_vec3();
        let forward = camera.direction;
        let right = forward.cross(up).normalize();

        // Accumulate movement.
        let mut movement = Vec3::ZERO;

        // Forward/backward.
        if keyboard.pressed(KeyCode::KeyW) {
            movement += forward;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            movement -= forward;
        }

        // Strafe left/right.
        if keyboard.pressed(KeyCode::KeyA) {
            movement -= right;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            movement += right;
        }

        // Ascend/descend.
        if keyboard.pressed(KeyCode::Space) {
            movement += up;
        }
        if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
            movement -= up;
        }

        if movement != Vec3::ZERO {
            movement = movement.normalize() * speed * time.delta_secs();

            // Apply movement to high-precision position.
            let movement_dvec = DVec3::new(
                f64::from(movement.x),
                f64::from(movement.y),
                f64::from(movement.z),
            );
            let new_position = origin_camera.position + movement_dvec;
            let new_altitude = new_position.length() - settings.earth_radius;

            // Prevent going too far from Earth or below surface.
            if new_altitude < 10_000_000.0 && new_altitude > -100.0 {
                origin_camera.position = new_position;
            }
        }
    }
}

/// Sync the floating origin resource with the camera position.
fn sync_floating_origin(mut origin: ResMut<FloatingOrigin>, query: Query<&FloatingOriginCamera>) {
    if let Ok(camera) = query.single() {
        origin.position = camera.position;
    }
}
