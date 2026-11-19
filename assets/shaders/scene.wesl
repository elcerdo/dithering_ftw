//! Scene pass: renders the 8-bit grayscale source image along with per object data.
//! r = luminance, g = dither pattern selector, b = region id, a = view depth.

import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
};
import super::dither_common::pattern_threshold;

struct SceneParams {
    region: f32,
    pattern: f32,
    stripes: vec2<f32>,
    texel_scale: f32,
    texel_mode: u32,
}

const TEXEL_MODE_OFF: u32 = 0u;
const TEXEL_MODE_THRESHOLD: u32 = 1u;
const TEXEL_MODE_PATTERN: u32 = 2u;

// region ids must stay exact integers in the 16-bit float render target
const REGION_MODULO: f32 = 2039.0;
const STRIPE_REGION_STEP: f32 = 61.0;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> params: SceneParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var bayer_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var blue_noise_texture: texture_2d<f32>;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color
    );
    let lit = apply_pbr_lighting(pbr_input);
    var luma = clamp(dot(lit.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.0, 1.0);

    var uv = vec2<f32>(0.0);
    @if(VERTEX_UVS_A) {
        uv = in.uv;
    }

    // sub regions for the wireframe lines, e.g. barrel staves
    let stripe = floor(fract(uv) * params.stripes);
    let stripe_index = stripe.x + stripe.y * params.stripes.x;
    let region = (params.region + stripe_index * STRIPE_REGION_STEP) % REGION_MODULO;

    // texel space mapping: threshold while rendering the scene, using the mesh uvs
    if params.texel_mode != TEXEL_MODE_OFF {
        let threshold = pattern_threshold(
            bayer_texture,
            blue_noise_texture,
            uv * params.texel_scale,
            params.pattern > 0.5,
        );
        if params.texel_mode == TEXEL_MODE_THRESHOLD {
            luma = select(0.0, 1.0, luma > threshold);
        } else if params.texel_mode == TEXEL_MODE_PATTERN {
            luma = threshold;
        } else {
            luma = select(0.0, 1.0, 0.5 > threshold);
        }
    }

    let view_depth = -(view.view_from_world * in.world_position).z;

    var out: FragmentOutput;
    out.color = vec4<f32>(luma, params.pattern, region, view_depth);
    return out;
}
