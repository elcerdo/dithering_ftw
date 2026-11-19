//! The devlog test scene: a corner of a ship cabin with a chair, a sphere on a stand,
//! a barrel, a bench, a framed picture, a sea chest and a wall shelf.

mod props;

use crate::global_state::GlobalState;
use crate::material::scene_material::SceneMaterial;
use crate::patterns::DitherPatterns;

use bevy::prelude::*;

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

const ROOM_MIN: Vec3 = Vec3::new(-2.5, 0.0, -2.5);
const ROOM_MAX: Vec3 = Vec3::new(3.5, 2.6, 3.5);
const LIGHT_POSITION: Vec3 = Vec3::new(0.6, 2.2, -1.5);
const LIGHT_INTENSITY: f32 = 50_000.0;
const AMBIENT_BRIGHTNESS: f32 = 4.0;
const SPHERE_SWING: f32 = 0.3;
const SPHERE_SWING_SPEED: f32 = 1.1;

//////////////////////////////////////////////////////////////////////

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SceneSettings { move_sphere: false });
        app.insert_resource(GlobalAmbientLight {
            brightness: AMBIENT_BRIGHTNESS,
            ..default()
        });
        app.add_systems(
            OnEnter(GlobalState::Ready),
            (populate_scene, populate_lights).chain(),
        );
        app.add_systems(Update, animate_sphere.run_if(in_state(GlobalState::Ready)));
    }
}

#[derive(Resource)]
pub struct SceneSettings {
    /// Swing the sphere sideways, as in the "Hold Still Please" capture.
    pub move_sphere: bool,
}

#[derive(Component)]
struct AnimatedSphere {
    origin: Vec3,
    phase: f32,
}

//////////////////////////////////////////////////////////////////////

fn populate_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SceneMaterial>>,
    patterns: Res<DitherPatterns>,
) {
    info!("** populate_scene **");

    let mut builder = props::Builder {
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        patterns: &patterns,
        next_region: 1,
    };

    props::populate_room(&mut builder, ROOM_MIN, ROOM_MAX);

    let sphere_origin = vec3(-1.7, 0.0, -1.7);
    let sphere_stand =
        props::populate_sphere_stand(&mut builder, Transform::from_translation(sphere_origin));
    builder
        .commands
        .entity(sphere_stand)
        .insert(AnimatedSphere {
            origin: sphere_origin,
            phase: 0.0,
        });

    props::populate_chair(
        &mut builder,
        Transform::from_xyz(-1.3, 0.0, -1.3).with_rotation(Quat::from_rotation_y(FRAC_PI_4)),
    );
    props::populate_barrel(&mut builder, Transform::from_xyz(-2.0, 0.0, -0.9));
    props::populate_bench(&mut builder, Transform::from_xyz(-2.12, 0.0, 0.25));
    props::populate_frame(
        &mut builder,
        Transform::from_xyz(ROOM_MIN.x, 1.75, -0.55)
            .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
    );
    props::populate_chest(&mut builder, Transform::from_xyz(-0.6, 0.0, -2.12));
    props::populate_shelf(&mut builder, Transform::from_xyz(-0.3, 1.95, ROOM_MIN.z));

    info!("num_regions {}", builder.next_region - 1);
}

fn populate_lights(mut commands: Commands) {
    // hanging lantern, high on the right side of the cabin
    commands.spawn((
        PointLight {
            color: Color::WHITE,
            intensity: LIGHT_INTENSITY,
            range: 20.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(LIGHT_POSITION),
    ));
}

fn animate_sphere(
    time: Res<Time>,
    settings: Res<SceneSettings>,
    mut spheres: Query<(&mut AnimatedSphere, &mut Transform)>,
) {
    let sideways = vec3(1.0, 0.0, -1.0).normalize();
    for (mut sphere, mut transform) in &mut spheres {
        if settings.move_sphere {
            sphere.phase += SPHERE_SWING_SPEED * time.delta_secs();
        }
        transform.translation = sphere.origin + sideways * SPHERE_SWING * sphere.phase.sin();
    }
}
