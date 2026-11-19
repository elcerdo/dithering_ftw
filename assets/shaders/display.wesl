//! Display pass: upscales the low resolution output to the window and applies the palette.

import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct DisplayParams {
    color_dark: vec4<f32>,
    color_light: vec4<f32>,
    source_size: vec2<f32>,
    texel_scale: f32,
    sharp_bilinear: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: DisplayParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var source_texture: texture_2d<f32>;

fn load_value(pixel: vec2<i32>) -> f32 {
    let size = vec2<i32>(params.source_size);
    return textureLoad(source_texture, clamp(pixel, vec2<i32>(0), size - 1), 0).r;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let position = mesh.uv * params.source_size;
    var value = 0.0;
    if params.sharp_bilinear == 0u {
        value = load_value(vec2<i32>(floor(position)));
    } else {
        // sharp bilinear: blend only across the window pixels straddling a source pixel boundary
        let shifted = position - 0.5;
        let base = floor(shifted);
        let fraction = clamp((shifted - base - 0.5) * params.texel_scale + 0.5, vec2<f32>(0.0), vec2<f32>(1.0));
        let pixel = vec2<i32>(base);
        let top = mix(load_value(pixel), load_value(pixel + vec2<i32>(1, 0)), fraction.x);
        let bottom = mix(load_value(pixel + vec2<i32>(0, 1)), load_value(pixel + vec2<i32>(1, 1)), fraction.x);
        value = mix(top, bottom, fraction.y);
    }
    return mix(params.color_dark, params.color_light, value);
}
