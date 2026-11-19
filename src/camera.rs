//! First person camera, Obra Dinn style: mostly looking around, slowly walking.

use bevy::prelude::*;

use bevy::input::mouse::AccumulatedMouseMotion;

const MOVE_SPEED: f32 = 1.5;
const KEY_TURN_SPEED: f32 = 1.2;
const MOUSE_SENSITIVITY: f32 = 0.003;
const PITCH_LIMIT: f32 = 1.4;
const ROOM_MIN: Vec2 = Vec2::new(-2.1, -2.1);
const ROOM_MAX: Vec2 = Vec2::new(3.1, 3.1);
const EYE_HEIGHT: f32 = 1.45;
const PAN_AMPLITUDE: f32 = 0.35;
const PAN_SPEED: f32 = 0.7;

//////////////////////////////////////////////////////////////////////

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraSettings { auto_pan: true });
        app.add_systems(Update, update_camera);
    }
}

#[derive(Resource)]
pub struct CameraSettings {
    /// Slow back and forth panning like the devlog captures.
    pub auto_pan: bool,
}

#[derive(Component)]
pub struct FirstPersonCamera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pan_phase: f32,
}

impl Default for FirstPersonCamera {
    /// Looking into the cabin corner with the chair and the sphere.
    fn default() -> Self {
        let position = vec3(0.25, EYE_HEIGHT, 0.25);
        let direction = vec3(-1.5, 0.95, -1.5) - position;
        Self {
            position,
            yaw: (-direction.x).atan2(-direction.z),
            pitch: (direction.y / direction.length()).asin(),
            pan_phase: 0.0,
        }
    }
}

impl FirstPersonCamera {
    fn transform(&self) -> Transform {
        let pan = PAN_AMPLITUDE * self.pan_phase.sin();
        let tilt = 0.25 * PAN_AMPLITUDE * (0.7 * self.pan_phase).sin();
        Transform::from_translation(self.position).with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            self.yaw + pan,
            self.pitch + tilt,
            0.0,
        ))
    }
}

pub fn update_camera(
    time: Res<Time>,
    settings: Res<CameraSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut cameras: Query<(&mut FirstPersonCamera, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (mut camera, mut transform) in &mut cameras {
        // look around
        let mut turn = Vec2::ZERO;
        if keyboard.pressed(KeyCode::ArrowLeft) {
            turn.x += KEY_TURN_SPEED * dt;
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            turn.x -= KEY_TURN_SPEED * dt;
        }
        if keyboard.pressed(KeyCode::ArrowUp) {
            turn.y += KEY_TURN_SPEED * dt;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            turn.y -= KEY_TURN_SPEED * dt;
        }
        if mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Middle) {
            turn -= mouse_motion.delta * MOUSE_SENSITIVITY;
        }
        camera.yaw += turn.x;
        camera.pitch = (camera.pitch + turn.y).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        // walk
        let mut walk = Vec2::ZERO;
        if keyboard.pressed(KeyCode::KeyW) {
            walk.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            walk.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            walk.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            walk.x += 1.0;
        }
        if walk != Vec2::ZERO {
            let rotation = Quat::from_rotation_y(camera.yaw);
            let forward = rotation * Vec3::NEG_Z;
            let right = rotation * Vec3::X;
            let delta = (forward * walk.y + right * walk.x).normalize() * MOVE_SPEED * dt;
            let position = (camera.position.xz() + delta.xz()).clamp(ROOM_MIN, ROOM_MAX);
            camera.position = vec3(position.x, EYE_HEIGHT, position.y);
        }

        if settings.auto_pan {
            camera.pan_phase += PAN_SPEED * dt;
        }

        *transform = camera.transform();
    }
}
