//! Regenerate assets/models/*.glb with `cargo run --release --example export_props`.
//! Geometry, UVs and part looks are exported from the original procedural builders.

#[path = "../src/material/scene_material.rs"]
pub mod scene_material;
mod material {
    pub use crate::scene_material;
}
#[path = "../src/patterns.rs"]
#[allow(dead_code)]
mod patterns;
#[path = "export_props/props.rs"]
mod props;

use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use serde_json::{Value, json};
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let output = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("models");
    std::fs::create_dir_all(&output)?;
    for name in [
        "room",
        "post",
        "sphere_stand",
        "chair",
        "barrel",
        "bench",
        "frame",
        "chest",
        "shelf",
    ] {
        let mut world = World::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<scene_material::SceneMaterial>::default();
        let patterns = patterns::DitherPatterns {
            bayer: Handle::default(),
            blue_noise: Handle::default(),
            grime: Handle::default(),
        };
        let mut queue = bevy::ecs::world::CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            let mut builder = props::Builder {
                commands: &mut commands,
                meshes: &mut meshes,
                materials: &mut materials,
                patterns: &patterns,
                next_region: 1,
            };
            match name {
                "room" => {
                    props::populate_room(&mut builder, vec3(-3.0, 0.0, -2.5), vec3(3.5, 2.6, 3.5))
                }
                "post" => props::populate_post(&mut builder, Transform::IDENTITY, 2.6),
                "sphere_stand" => {
                    props::populate_sphere_stand(&mut builder, Transform::IDENTITY);
                }
                "chair" => props::populate_chair(&mut builder, Transform::IDENTITY),
                "barrel" => props::populate_barrel(&mut builder, Transform::IDENTITY),
                "bench" => props::populate_bench(&mut builder, Transform::IDENTITY),
                "frame" => props::populate_frame(&mut builder, Transform::IDENTITY),
                "chest" => props::populate_chest(&mut builder, Transform::IDENTITY),
                "shelf" => props::populate_shelf(&mut builder, Transform::IDENTITY),
                _ => unreachable!(),
            }
        }
        queue.apply(&mut world);
        let mut glb = Glb::default();
        let mut query = world.query::<(
            &Mesh3d,
            &MeshMaterial3d<scene_material::SceneMaterial>,
            &Transform,
            Has<NotShadowCaster>,
        )>();
        for (mesh, material, transform, not_shadow_caster) in query.iter(&world) {
            glb.part(
                meshes.get(&mesh.0).expect("Generated mesh"),
                materials.get(&material.0).expect("Generated material"),
                transform,
                not_shadow_caster,
            );
        }
        let path = output.join(format!("{name}.glb"));
        glb.save(&path)?;
        println!("Exported {}", path.display());
    }
    Ok(())
}

#[derive(Default)]
struct Glb {
    binary: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
    meshes: Vec<Value>,
    materials: Vec<Value>,
    nodes: Vec<Value>,
}

impl Glb {
    fn accessor(
        &mut self,
        bytes: Vec<u8>,
        count: usize,
        component_type: u32,
        kind: &str,
        bounds: Option<([f32; 3], [f32; 3])>,
    ) -> usize {
        let view = self.views.len();
        self.views.push(json!({
            "buffer": 0, "byteOffset": self.binary.len(), "byteLength": bytes.len(),
        }));
        self.binary.extend(bytes);
        let mut accessor = json!({
            "bufferView": view, "componentType": component_type, "count": count, "type": kind,
        });
        if let Some((min, max)) = bounds {
            accessor["min"] = json!(min);
            accessor["max"] = json!(max);
        }
        let index = self.accessors.len();
        self.accessors.push(accessor);
        index
    }

    fn part(
        &mut self,
        mesh: &Mesh,
        material: &scene_material::SceneMaterial,
        transform: &Transform,
        not_shadow_caster: bool,
    ) {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("Expected float3 positions");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("Expected float3 normals");
        };
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("Expected float2 UVs");
        };
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for position in positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        let position = self.accessor(
            float_bytes(positions.iter().flatten().copied()),
            positions.len(),
            5126,
            "VEC3",
            Some((min, max)),
        );
        let normal = self.accessor(
            float_bytes(normals.iter().flatten().copied()),
            normals.len(),
            5126,
            "VEC3",
            None,
        );
        let uv = self.accessor(
            float_bytes(uvs.iter().flatten().copied()),
            uvs.len(),
            5126,
            "VEC2",
            None,
        );
        let indices: Vec<u32> = match mesh.indices().expect("Indexed mesh") {
            Indices::U16(values) => values.iter().map(|&value| u32::from(value)).collect(),
            Indices::U32(values) => values.clone(),
        };
        let indices = self.accessor(
            indices
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect(),
            indices.len(),
            5125,
            "SCALAR",
            None,
        );
        let color = material.base.base_color.to_linear();
        let params = material.extension.params;
        let material_index = self.materials.len();
        self.materials.push(json!({
            "pbrMetallicRoughness": {
                "baseColorFactor": [color.red, color.green, color.blue, color.alpha],
                "metallicFactor": material.base.metallic,
                "roughnessFactor": material.base.perceptual_roughness,
            },
        }));
        let mesh_index = self.meshes.len();
        self.meshes.push(json!({
            "primitives": [{
                "attributes": {"POSITION": position, "NORMAL": normal, "TEXCOORD_0": uv},
                "indices": indices, "material": material_index, "mode": 4,
            }],
        }));
        let grime_scale = material.base.base_color_texture.as_ref().map(|_| {
            [
                material.base.uv_transform.matrix2.x_axis.x,
                material.base.uv_transform.matrix2.y_axis.y,
            ]
        });
        self.nodes.push(json!({
            "name": format!("part_{}", self.nodes.len()),
            "mesh": mesh_index,
            "translation": transform.translation.to_array(),
            "rotation": transform.rotation.to_array(),
            "scale": transform.scale.to_array(),
            "extras": {
                "gray": color.red,
                "roughness": material.base.perceptual_roughness,
                "bayer": params.pattern == 1.0,
                "stripes": params.stripes.to_array(),
                "texel_scale": params.texel_scale,
                "grime_scale": grime_scale,
                "not_shadow_caster": not_shadow_caster,
            },
        }));
    }

    fn save(self, path: &Path) -> Result<(), Box<dyn Error>> {
        let document = json!({
            "asset": {"version": "2.0", "generator": "dithering_ftw export_props"},
            "scene": 0,
            "scenes": [{"nodes": (0..self.nodes.len()).collect::<Vec<_>>()}],
            "nodes": self.nodes,
            "meshes": self.meshes,
            "materials": self.materials,
            "accessors": self.accessors,
            "bufferViews": self.views,
            "buffers": [{"byteLength": self.binary.len()}],
        });
        let mut json = serde_json::to_vec(&document)?;
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let length = u32::try_from(12 + 8 + json.len() + 8 + self.binary.len())?;
        let mut bytes = Vec::new();
        bytes.extend(b"glTF");
        bytes.extend(2u32.to_le_bytes());
        bytes.extend(length.to_le_bytes());
        bytes.extend(u32::try_from(json.len())?.to_le_bytes());
        bytes.extend(b"JSON");
        bytes.extend(json);
        bytes.extend(u32::try_from(self.binary.len())?.to_le_bytes());
        bytes.extend(b"BIN\0");
        bytes.extend(self.binary);
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

fn float_bytes(values: impl Iterator<Item = f32>) -> Vec<u8> {
    values.flat_map(f32::to_le_bytes).collect()
}
