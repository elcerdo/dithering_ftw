//! Offline geometry builders for the saved cabin props.
//! Every part gets its own region id so the dither pass can draw wireframe lines between them.

use crate::material::scene_material::{self, ObjectPattern, SceneLook, SceneMaterial};
use crate::patterns::DitherPatterns;

use bevy::prelude::*;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

const WOOD: f32 = 0.45;
const DARK_WOOD: f32 = 0.3;
const IRON: f32 = 0.15;
const BRASS: f32 = 0.7;
/// Pulls faces of overlapping parts apart so they don't z-fight.
const INSET: f32 = 0.004;

//////////////////////////////////////////////////////////////////////

pub struct Builder<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<SceneMaterial>,
    pub patterns: &'a DitherPatterns,
    pub next_region: u32,
}

impl Builder<'_, '_, '_> {
    pub fn root(&mut self, transform: Transform) -> Entity {
        self.commands.spawn((transform, Visibility::default())).id()
    }

    pub fn part(
        &mut self,
        parent: Option<Entity>,
        mesh: impl Into<Mesh>,
        transform: Transform,
        look: &SceneLook,
    ) -> Entity {
        let region = self.next_region;
        self.next_region += 1;
        let material = self
            .materials
            .add(scene_material::make(self.patterns, look, region));
        let mesh = self.meshes.add(mesh.into());
        let mut entity = self
            .commands
            .spawn((Mesh3d(mesh), MeshMaterial3d(material), transform));
        if let Some(parent) = parent {
            entity.insert(ChildOf(parent));
        }
        entity.id()
    }

    /// Room shell part, it doesn't cast shadows.
    pub fn shell_part(
        &mut self,
        mesh: impl Into<Mesh>,
        transform: Transform,
        look: &SceneLook,
    ) -> Entity {
        let entity = self.part(None, mesh, transform, look);
        self.commands.entity(entity).insert(NotShadowCaster);
        entity
    }
}

fn wood(gray: f32) -> SceneLook {
    SceneLook { gray, ..default() }
}

//////////////////////////////////////////////////////////////////////

/// Back wall facing the camera plus the rest of the cabin to close it.
pub fn populate_room(builder: &mut Builder, room_min: Vec3, room_max: Vec3) {
    let size = room_max - room_min;
    let center = (room_min + room_max) / 2.0;
    let grime = builder.patterns.grime.clone();
    let wall_look = |width: f32| SceneLook {
        gray: 0.8,
        roughness: 1.0,
        texture: Some((grime.clone(), vec2(width / 3.0, 1.0))),
        texel_scale: 48.0 * width,
        ..default()
    };

    let walls = [
        // back wall, facing +z
        (Transform::from_xyz(center.x, center.y, room_min.z), size.x),
        // left wall, facing +x
        (
            Transform::from_xyz(room_min.x, center.y, center.z)
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
            size.z,
        ),
        // right wall, facing -x
        (
            Transform::from_xyz(room_max.x, center.y, center.z)
                .with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
            size.z,
        ),
        // front wall, facing -z
        (
            Transform::from_xyz(center.x, center.y, room_max.z)
                .with_rotation(Quat::from_rotation_y(PI)),
            size.x,
        ),
    ];
    for (transform, width) in walls {
        let look = wall_look(width);
        builder.shell_part(Rectangle::new(width, size.y), transform, &look);
    }

    // ceiling and beams
    builder.shell_part(
        Rectangle::new(size.x, size.z),
        Transform::from_xyz(center.x, room_max.y, center.z)
            .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        &wood(0.35),
    );
    for beam_z in [-1.4, 0.2, 1.8] {
        builder.shell_part(
            Cuboid::new(size.x, 0.16, 0.14),
            Transform::from_xyz(center.x, room_max.y - 0.08, beam_z),
            &wood(DARK_WOOD),
        );
    }

    // floor planks running toward the back wall with staggered joints
    let plank_width = 0.2;
    let num_planks = (size.x / plank_width).ceil() as usize;
    for index in 0..num_planks {
        let xx = room_min.x + (index as f32 + 0.5) * plank_width;
        let joint = room_min.z + size.z * (0.25 + 0.5 * ((index * 7) % 5) as f32 / 4.0);
        for (start, end) in [(room_min.z, joint), (joint, room_max.z)] {
            let gray = 0.5 + 0.08 * (((index * 13 + (end * 10.0) as usize) % 5) as f32 / 4.0 - 0.5);
            builder.shell_part(
                Cuboid::new(plank_width - 0.006, 0.04, end - start - 0.004),
                Transform::from_xyz(xx, -0.02, (start + end) / 2.0),
                &SceneLook {
                    gray,
                    texel_scale: 64.0 * (end - start),
                    ..default()
                },
            );
        }
    }
}

/// Vertical wooden post against a wall, front facing local +z.
pub fn populate_post(builder: &mut Builder, transform: Transform, height: f32) {
    builder.part(
        None,
        Cuboid::new(0.14, height, 0.06),
        transform * Transform::from_xyz(0.0, height / 2.0, 0.03),
        &SceneLook {
            gray: 0.6,
            texel_scale: 48.0 * height,
            ..default()
        },
    );
}

/// The sphere on a stand, the only object dithered with the bayer matrix.
pub fn populate_sphere_stand(builder: &mut Builder, transform: Transform) -> Entity {
    let root = builder.root(transform);
    let iron = wood(IRON);
    let radius = 0.37;
    let center_height = 1.35;
    builder.part(
        Some(root),
        Cylinder::new(0.14, 0.03),
        Transform::from_xyz(0.0, 0.015, 0.0),
        &iron,
    );
    builder.part(
        Some(root),
        Cylinder::new(0.012, center_height),
        Transform::from_xyz(0.0, center_height / 2.0, 0.0),
        &iron,
    );
    builder.part(
        Some(root),
        Sphere::new(radius).mesh().uv(48, 24),
        Transform::from_xyz(0.0, center_height, 0.0),
        &SceneLook {
            gray: 0.5,
            roughness: 0.8,
            pattern: ObjectPattern::Bayer,
            texel_scale: 96.0,
            ..default()
        },
    );
    root
}

/// Simple wooden chair, front facing local +z.
pub fn populate_chair(builder: &mut Builder, transform: Transform) {
    let root = builder.root(transform);
    let look = wood(WOOD);
    let seat_height = 0.45;
    builder.part(
        Some(root),
        Cuboid::new(0.46, 0.05, 0.44),
        Transform::from_xyz(0.0, seat_height, 0.0),
        &look,
    );
    for (xx, zz) in [(-0.2, -0.19), (0.2, -0.19), (-0.2, 0.19), (0.2, 0.19)] {
        builder.part(
            Some(root),
            Cuboid::new(0.045, seat_height, 0.045),
            Transform::from_xyz(xx, seat_height / 2.0, zz),
            &look,
        );
    }
    for xx in [-0.2, 0.2] {
        builder.part(
            Some(root),
            Cuboid::new(0.045, 0.52, 0.045),
            Transform::from_xyz(xx, seat_height + 0.26, -0.19),
            &look,
        );
    }
    builder.part(
        Some(root),
        Cuboid::new(0.44, 0.1, 0.035),
        Transform::from_xyz(0.0, seat_height + 0.47, -0.19),
        &look,
    );
    builder.part(
        Some(root),
        Cuboid::new(0.3, 0.2, 0.02),
        Transform::from_xyz(0.0, seat_height + 0.25, -0.19),
        &wood(DARK_WOOD),
    );
    builder.part(
        Some(root),
        Cuboid::new(0.36, 0.035, 0.03),
        Transform::from_xyz(0.0, 0.15, 0.19),
        &look,
    );
}

/// Bulging barrel with staves and iron hoops.
pub fn populate_barrel(builder: &mut Builder, transform: Transform) {
    let root = builder.root(transform);
    let height = 0.85;
    let radius_end = 0.30;
    let radius_mid = 0.36;
    let radius_at = |yy: f32| radius_end + (radius_mid - radius_end) * (PI * yy / height).sin();
    builder.part(
        Some(root),
        make_barrel_mesh(height, radius_end, radius_mid, 36, 16),
        Transform::IDENTITY,
        &SceneLook {
            gray: WOOD,
            stripes: vec2(18.0, 0.0),
            texel_scale: 128.0,
            ..default()
        },
    );
    builder.part(
        Some(root),
        Cylinder::new(radius_end - 0.015, 0.02),
        Transform::from_xyz(0.0, height - 0.03, 0.0),
        &wood(DARK_WOOD),
    );
    for yy in [0.07, 0.24, 0.61, 0.78] {
        builder.part(
            Some(root),
            Cylinder::new(radius_at(yy) + 0.006, 0.035),
            Transform::from_xyz(0.0, yy, 0.0),
            &wood(IRON),
        );
    }
}

/// Long bench along local z.
pub fn populate_bench(builder: &mut Builder, transform: Transform) {
    let root = builder.root(transform);
    let look = wood(WOOD);
    builder.part(
        Some(root),
        Cuboid::new(0.5, 0.06, 1.3),
        Transform::from_xyz(0.0, 0.48, 0.0),
        &look,
    );
    for (xx, zz) in [(-0.19, -0.55), (0.19, -0.55), (-0.19, 0.55), (0.19, 0.55)] {
        builder.part(
            Some(root),
            Cuboid::new(0.06, 0.45, 0.06),
            Transform::from_xyz(xx, 0.225, zz),
            &look,
        );
    }
    builder.part(
        Some(root),
        Cuboid::new(0.04, 0.05, 1.1),
        Transform::from_xyz(0.0, 0.12, 0.0),
        &wood(DARK_WOOD),
    );
}

/// Framed picture hanging on a wall, front facing local +z.
pub fn populate_frame(builder: &mut Builder, transform: Transform) {
    let root = builder.root(transform);
    let look = wood(DARK_WOOD);
    let (width, height, border) = (0.9, 0.75, 0.07);
    for yy in [-1.0, 1.0] {
        builder.part(
            Some(root),
            Cuboid::new(width, border, 0.04),
            Transform::from_xyz(0.0, yy * (height - border) / 2.0, 0.02),
            &look,
        );
    }
    for xx in [-1.0, 1.0] {
        builder.part(
            Some(root),
            Cuboid::new(border, height - 2.0 * border, 0.04),
            Transform::from_xyz(xx * (width - border) / 2.0, 0.0, 0.02),
            &look,
        );
    }
    builder.part(
        Some(root),
        Cuboid::new(width - 2.0 * border, height - 2.0 * border, 0.01),
        Transform::from_xyz(0.0, 0.0, 0.005),
        &wood(0.65),
    );
}

/// Sea chest with a curved lid, iron straps and a lock, front facing local +z.
pub fn populate_chest(builder: &mut Builder, transform: Transform) {
    let root = builder.root(transform);
    let (width, height, depth) = (1.3, 0.45, 0.6);
    let lid_rotation = Quat::from_rotation_z(FRAC_PI_2);
    builder.part(
        Some(root),
        Cuboid::new(width, height, depth),
        Transform::from_xyz(0.0, height / 2.0, 0.0),
        &wood(0.2),
    );
    // the lower half of the lid cylinder hides inside the box, its caps stay inside the box sides
    builder.part(
        Some(root),
        Cylinder::new(depth / 2.0, width - 2.0 * INSET),
        Transform::from_xyz(0.0, height, 0.0).with_rotation(lid_rotation),
        &wood(0.45),
    );
    for xx in [-0.35, 0.35] {
        builder.part(
            Some(root),
            Cuboid::new(0.07, height + 0.005, depth + 0.012),
            Transform::from_xyz(xx, height / 2.0, 0.0),
            &wood(BRASS),
        );
        builder.part(
            Some(root),
            Cylinder::new(depth / 2.0 + 0.006, 0.07 - 2.0 * INSET),
            Transform::from_xyz(xx, height, 0.0).with_rotation(lid_rotation),
            &wood(BRASS),
        );
        // hasp hanging over the strap
        builder.part(
            Some(root),
            Cuboid::new(0.05, 0.1, 0.02),
            Transform::from_xyz(xx, height - 0.1, depth / 2.0 + 0.016),
            &wood(BRASS),
        );
    }
    for xx in [-1.0, 1.0] {
        builder.part(
            Some(root),
            Cuboid::new(0.03, 0.05, 0.18),
            Transform::from_xyz(xx * (width / 2.0 + 0.015), height - 0.12, 0.0),
            &wood(IRON),
        );
    }
}

/// Wall shelf with brackets and a few items, back against local -z.
pub fn populate_shelf(builder: &mut Builder, transform: Transform) {
    let root = builder.root(transform);
    let look = wood(WOOD);
    builder.part(
        Some(root),
        Cuboid::new(1.2, 0.04, 0.3),
        Transform::from_xyz(0.0, 0.0, 0.15),
        &look,
    );
    for xx in [-0.45, 0.45] {
        builder.part(
            Some(root),
            Cuboid::new(0.04, 0.3, 0.03),
            Transform::from_xyz(xx, -0.17, 0.015),
            &look,
        );
        builder.part(
            Some(root),
            Cuboid::new(0.03, 0.34, 0.03),
            Transform::from_xyz(xx, -0.13, 0.12).with_rotation(Quat::from_rotation_x(FRAC_PI_4)),
            &look,
        );
    }
    builder.part(
        Some(root),
        Cuboid::new(0.3, 0.18, 0.22),
        Transform::from_xyz(-0.25, 0.11, 0.15),
        &wood(0.55),
    );
    builder.part(
        Some(root),
        Cylinder::new(0.045, 0.22),
        Transform::from_xyz(0.25, 0.13, 0.15),
        &SceneLook {
            gray: 0.6,
            roughness: 0.3,
            ..default()
        },
    );
}

//////////////////////////////////////////////////////////////////////

/// Lathe mesh with a sine bulge, uv.x goes around and uv.y goes down.
fn make_barrel_mesh(
    height: f32,
    radius_end: f32,
    radius_mid: f32,
    num_sides: usize,
    num_rings: usize,
) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for ring in 0..=num_rings {
        let tt = ring as f32 / num_rings as f32;
        let yy = tt * height;
        let radius = radius_end + (radius_mid - radius_end) * (PI * tt).sin();
        let slope = (radius_mid - radius_end) * PI * (PI * tt).cos() / height;
        for side in 0..=num_sides {
            let ss = side as f32 / num_sides as f32;
            let (sin, cos) = (ss * TAU).sin_cos();
            positions.push([radius * cos, yy, radius * sin]);
            normals.push(vec3(cos, -slope, sin).normalize().to_array());
            uvs.push([ss, 1.0 - tt]);
        }
    }
    let mut indices = Vec::new();
    let stride = (num_sides + 1) as u32;
    for ring in 0..num_rings as u32 {
        for side in 0..num_sides as u32 {
            let aa = ring * stride + side;
            let bb = aa + 1;
            let cc = aa + stride;
            let dd = cc + 1;
            indices.extend([aa, cc, bb, bb, cc, dd]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}
