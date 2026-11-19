//! Display pass: integer upscales the low resolution output to the window and applies the palette.

import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct DisplayParams {
    color_dark: vec4<f32>,
    color_light: vec4<f32>,
    source_size: vec2<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: DisplayParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var source_texture: texture_2d<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(params.source_size);
    let pixel = clamp(vec2<i32>(floor(mesh.uv * params.source_size)), vec2<i32>(0), size - 1);
    let value = textureLoad(source_texture, pixel, 0).r;
    return mix(params.color_dark, params.color_light, value);
}
