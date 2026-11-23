//! Saved GLB props, with per-part dither settings stored in node extras.

use crate::material::scene_material::{self, ObjectPattern, SceneLook, SceneMaterial};
use crate::patterns::DitherPatterns;

use bevy::asset::LoadState;
use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use serde::Deserialize;

const PROP_NAMES: [&str; 9] = [
    "room",
    "post",
    "sphere_stand",
    "chair",
    "barrel",
    "bench",
    "frame",
    "chest",
    "shelf",
];

#[derive(Resource)]
pub struct PropAssets {
    handles: Vec<Handle<Gltf>>,
}

pub fn load_props(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(PropAssets {
        handles: PROP_NAMES
            .iter()
            .map(|name| server.load(format!("models/{name}.glb")))
            .collect(),
    });
}

pub fn props_ready(server: Res<AssetServer>, props: Res<PropAssets>) -> bool {
    props.handles.iter().all(|handle| {
        if let Some(LoadState::Failed(error)) = server.get_load_state(handle.id()) {
            panic!("Failed to load cabin prop {:?}: {error}", handle.path());
        }
        server.is_loaded_with_dependencies(handle.id())
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartLook {
    gray: f32,
    roughness: f32,
    bayer: bool,
    stripes: [f32; 2],
    texel_scale: f32,
    grime_scale: Option<[f32; 2]>,
    not_shadow_caster: bool,
}

pub struct Builder<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub props: &'a PropAssets,
    pub gltfs: &'a Assets<Gltf>,
    pub nodes: &'a Assets<GltfNode>,
    pub meshes: &'a Assets<GltfMesh>,
    pub materials: &'a mut Assets<SceneMaterial>,
    pub patterns: &'a DitherPatterns,
    pub next_region: u32,
}

impl Builder<'_, '_, '_> {
    pub fn spawn(&mut self, name: &str, transform: Transform) -> Entity {
        let index = PROP_NAMES
            .iter()
            .position(|&prop| prop == name)
            .expect("Unknown cabin prop");
        let gltf = self
            .gltfs
            .get(&self.props.handles[index])
            .expect("Loaded GLB");
        let root = self.commands.spawn((transform, Visibility::default())).id();
        // Exported props have one flat node per part, keeping separate wireframe regions.
        for handle in &gltf.nodes {
            let node = self.nodes.get(handle).expect("Loaded GLB node");
            assert!(node.children.is_empty(), "Cabin prop nodes must be flat");
            let mesh = self
                .meshes
                .get(node.mesh.as_ref().expect("Cabin part must have a mesh"))
                .expect("Loaded GLB mesh");
            let extras = node
                .extras
                .as_ref()
                .expect("Cabin part must have look extras");
            let part: PartLook =
                serde_json::from_str(&extras.value).expect("Invalid cabin part look extras");
            let look = SceneLook {
                gray: part.gray,
                roughness: part.roughness,
                pattern: if part.bayer {
                    ObjectPattern::Bayer
                } else {
                    ObjectPattern::BlueNoise
                },
                stripes: Vec2::from_array(part.stripes),
                texel_scale: part.texel_scale,
                texture: part
                    .grime_scale
                    .map(|scale| (self.patterns.grime.clone(), Vec2::from_array(scale))),
            };
            for primitive in &mesh.primitives {
                let material = self.materials.add(scene_material::make(
                    self.patterns,
                    &look,
                    self.next_region,
                ));
                self.next_region += 1;
                let mut entity = self.commands.spawn((
                    Mesh3d(primitive.mesh.clone()),
                    MeshMaterial3d(material),
                    node.transform,
                    ChildOf(root),
                ));
                if part.not_shadow_caster {
                    entity.insert(NotShadowCaster);
                }
            }
        }
        root
    }
}
