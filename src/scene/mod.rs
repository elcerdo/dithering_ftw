//! The devlog test scene: a ship cabin wall with a framed picture, a barrel, a sphere on a stand,
//! a chair, a sea chest, a wall shelf and a bench.

mod props;

use crate::global_state::GlobalState;
use crate::material::scene_material::SceneMaterial;
use crate::patterns::DitherPatterns;

use bevy::prelude::*;

const ROOM_MIN: Vec3 = Vec3::new(-3.0, 0.0, -2.5);
const ROOM_MAX: Vec3 = Vec3::new(3.5, 2.6, 3.5);
const LIGHT_POSITION: Vec3 = Vec3::new(0.5, 2.2, -1.2);
const LIGHT_INTENSITY: f32 = 4e4;
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
    for post_x in [-1.65, 0.8] {
        props::populate_post(
            &mut builder,
            Transform::from_xyz(post_x, 0.0, ROOM_MIN.z),
            ROOM_MAX.y,
        );
    }

    let sphere_origin = vec3(-0.4, 0.0, -1.95);
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
        Transform::from_xyz(-0.15, 0.0, -1.45).with_rotation(Quat::from_rotation_y(-0.35)),
    );
    props::populate_barrel(&mut builder, Transform::from_xyz(-1.3, 0.0, -1.75));
    props::populate_bench(&mut builder, Transform::from_xyz(-2.7, 0.0, -1.0));
    props::populate_frame(&mut builder, Transform::from_xyz(-2.25, 1.55, ROOM_MIN.z));
    props::populate_chest(&mut builder, Transform::from_xyz(1.3, 0.0, -2.15));
    props::populate_shelf(&mut builder, Transform::from_xyz(1.6, 1.6, ROOM_MIN.z));

    info!("num_regions {}", builder.next_region - 1);
}

fn populate_lights(mut commands: Commands) {
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
    for (mut sphere, mut transform) in &mut spheres {
        if settings.move_sphere {
            sphere.phase += SPHERE_SWING_SPEED * time.delta_secs();
        }
        transform.translation = sphere.origin + Vec3::X * SPHERE_SWING * sphere.phase.sin();
    }
}
