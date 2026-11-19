//! Post-processing pass: converts the 8-bit scene to 1-bit at the output resolution by comparing
//! each source pixel to a dither pattern. The pattern mappings follow the devlog experiments:
//! screen, screen with offset, texel space, motion warping, cube and sphere mapping.
//! An optional probe disc thresholds flat gray with the bayer matrix, like the inset in the
//! devlog captures, to show how the current mapping moves the pattern.
//!
//! Output: r = value, gb = motion warp residual, a = view depth.

import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;
import super::dither_common::pattern_threshold;

struct DitherParams {
    world_from_view: mat4x4<f32>,
    prev_clip_from_world: mat4x4<f32>,
    output_size: vec2<f32>,
    tan_half_fov: vec2<f32>,
    screen_offset: vec2<f32>,
    screen_offset_delta: vec2<f32>,
    probe_center: vec2<f32>,
    probe_radius: f32,
    probe_gray: f32,
    supersample: u32,
    mapping: u32,
    view_mode: u32,
    pattern_mode: u32,
    edges: u32,
    final_threshold: u32,
    reset: u32,
    probe: u32,
}

const MAPPING_SCREEN: u32 = 0u;
const MAPPING_SCREEN_OFFSET: u32 = 1u;
const MAPPING_TEXEL: u32 = 2u;
const MAPPING_MOTION_WARP: u32 = 3u;
const MAPPING_CUBE: u32 = 4u;
const MAPPING_SPHERE: u32 = 5u;

const VIEW_OUTPUT: u32 = 0u;
const VIEW_SOURCE: u32 = 1u;
const VIEW_PATTERN: u32 = 2u;
const VIEW_FLAT_GRAY: u32 = 3u;

const PATTERN_BAYER: u32 = 1u;
const PATTERN_BLUE_NOISE: u32 = 2u;

const PI: f32 = 3.141592653589793;
// residuals are stored modulo the largest pattern size to keep 16-bit float precision
const STATE_WRAP: f32 = 128.0;
const WARP_DEPTH_TOLERANCE: f32 = 0.05;
const SPHERE_BAND_ROWS: f32 = 8.0;
const LINE_BRIGHTNESS_THRESHOLD: f32 = 0.5;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: DitherParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var scene_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var bayer_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var blue_noise_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var previous_texture: texture_2d<f32>;

fn use_bayer(object_pattern: f32) -> bool {
    if params.pattern_mode == PATTERN_BAYER {
        return true;
    }
    if params.pattern_mode == PATTERN_BLUE_NOISE {
        return false;
    }
    return object_pattern > 0.5;
}

/// Position in output pixels.
fn in_probe(position: vec2<f32>) -> bool {
    return params.probe != 0u && length(position - params.probe_center) < params.probe_radius;
}

/// Wraps to [-STATE_WRAP / 2, STATE_WRAP / 2[, the patterns tile so this is lossless.
fn wrap_centered(value: vec2<f32>) -> vec2<f32> {
    return value - STATE_WRAP * floor(value / STATE_WRAP + 0.5);
}

fn load_scene(scene_pixel: vec2<i32>) -> vec4<f32> {
    let scene_size = vec2<i32>(params.output_size) * i32(params.supersample);
    return textureLoad(scene_texture, clamp(scene_pixel, vec2<i32>(0), scene_size - 1), 0);
}

/// World space view direction through a point of the scene image.
fn world_direction(scene_position: vec2<f32>, scene_size: vec2<f32>) -> vec3<f32> {
    let ndc = scene_position / scene_size * 2.0 - 1.0;
    let view_direction = vec3<f32>(
        ndc.x * params.tan_half_fov.x,
        -ndc.y * params.tan_half_fov.y,
        -1.0,
    );
    return normalize((params.world_from_view * vec4<f32>(view_direction, 0.0)).xyz);
}

/// Pattern pre-rendered on the faces of a world aligned cube centered on the camera.
/// One texel matches one pixel when looking straight into a face.
fn cube_coord(direction: vec3<f32>, texel_angle: f32) -> vec2<f32> {
    let magnitude = abs(direction);
    var face = vec2<f32>(0.0);
    if magnitude.x >= magnitude.y && magnitude.x >= magnitude.z {
        face = vec2<f32>(direction.z * sign(direction.x), -direction.y) / magnitude.x;
    } else if magnitude.y >= magnitude.z {
        face = vec2<f32>(direction.x, direction.z * sign(direction.y)) / magnitude.y;
    } else {
        face = vec2<f32>(-direction.x * sign(direction.z), -direction.y) / magnitude.z;
    }
    return face / texel_angle;
}

/// "Rings" mapping on the inside of a camera centered sphere: rows of constant elevation, each ring
/// holding a whole number of texels so that texels stay roughly pixel sized. Rows are grouped in
/// bands as tall as the bayer matrix sharing the same ring length, which keeps the matrix intact.
fn sphere_coord(direction: vec3<f32>, texel_angle: f32) -> vec2<f32> {
    let elevation = asin(clamp(direction.y, -1.0, 1.0));
    let azimuth = atan2(direction.x, -direction.z);
    let row_coord = -elevation / texel_angle;
    let band_elevation = (floor(row_coord / SPHERE_BAND_ROWS) + 0.5) * SPHERE_BAND_ROWS * texel_angle;
    let ring_length = max(SPHERE_BAND_ROWS, round(2.0 * PI * cos(band_elevation) / texel_angle));
    let column_coord = (azimuth / (2.0 * PI) + 0.5) * ring_length;
    return vec2<f32>(column_coord, row_coord);
}

/// Motion warping: track each pixel back to the previous frame and carry its pattern coordinate along.
/// The state is stored as a residual relative to the screen mapping with offset, which already follows
/// the camera rotation, so that reloaded pixels (not visible in the previous frame) stay coherent.
fn warp_residual(pixel: vec2<i32>, depth: f32) -> vec2<f32> {
    let center = vec2<f32>(pixel) + 0.5;
    if params.reset != 0u || depth <= 0.0 {
        return vec2<f32>(0.0);
    }
    let ndc = center / params.output_size * 2.0 - 1.0;
    let view_position = vec3<f32>(
        ndc.x * params.tan_half_fov.x,
        -ndc.y * params.tan_half_fov.y,
        -1.0,
    ) * depth;
    let world_position = params.world_from_view * vec4<f32>(view_position, 1.0);
    let prev_clip = params.prev_clip_from_world * world_position;
    if prev_clip.w <= 0.0 {
        return vec2<f32>(0.0);
    }
    let prev_ndc = prev_clip.xy / prev_clip.w;
    let prev_position = vec2<f32>(0.5 + 0.5 * prev_ndc.x, 0.5 - 0.5 * prev_ndc.y) * params.output_size;
    if any(prev_position < vec2<f32>(0.0)) || any(prev_position >= params.output_size) {
        return vec2<f32>(0.0);
    }
    // bilinear resampling of the previous residuals, only from taps on the same surface
    let base = prev_position - 0.5;
    let base_pixel = vec2<i32>(floor(base));
    let fraction = base - floor(base);
    let max_pixel = vec2<i32>(params.output_size) - 1;
    var reference = vec2<f32>(0.0);
    var residual_sum = vec2<f32>(0.0);
    var weight_sum = 0.0;
    for (var dy = 0; dy < 2; dy++) {
        for (var dx = 0; dx < 2; dx++) {
            let tap = clamp(base_pixel + vec2<i32>(dx, dy), vec2<i32>(0), max_pixel);
            let prev = textureLoad(previous_texture, tap, 0);
            let weights = select(1.0 - fraction, fraction, vec2<bool>(dx == 1, dy == 1));
            // for a perspective projection, clip w is the view depth
            if abs(prev.a - prev_clip.w) <= WARP_DEPTH_TOLERANCE * prev_clip.w {
                // undo the storage wrapping relative to the first valid tap before averaging
                if weight_sum == 0.0 {
                    reference = prev.gb;
                }
                let tap_residual = reference + wrap_centered(prev.gb - reference);
                residual_sum += weights.x * weights.y * tap_residual;
                weight_sum += weights.x * weights.y;
            }
        }
    }
    if weight_sum < 1e-3 {
        return vec2<f32>(0.0);
    }
    return residual_sum / weight_sum + prev_position - center + params.screen_offset_delta;
}

/// Wireframe lines are drawn where the region ids differ.
fn is_edge(pixel: vec2<i32>, region: f32) -> bool {
    let supersample = i32(params.supersample);
    let right = load_scene((pixel + vec2<i32>(1, 0)) * supersample).b;
    let down = load_scene((pixel + vec2<i32>(0, 1)) * supersample).b;
    return right != region || down != region;
}

/// Lines contrast with their surroundings: light over dark areas and dark over light ones.
fn line_value(pixel: vec2<i32>) -> f32 {
    let supersample = i32(params.supersample);
    var sum = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            sum += load_scene((pixel + vec2<i32>(dx, dy)) * supersample).r;
        }
    }
    return select(1.0, 0.0, sum / 9.0 > LINE_BRIGHTNESS_THRESHOLD);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(mesh.position.xy));
    let supersample = i32(params.supersample);
    let scene_size = params.output_size * f32(supersample);
    let texel_angle = 2.0 * params.tan_half_fov.y / scene_size.y;

    let center = load_scene(pixel * supersample);
    let depth = center.a;

    var residual = vec2<f32>(0.0);
    if params.mapping == MAPPING_MOTION_WARP {
        residual = warp_residual(pixel, depth);
    }

    // threshold every scene pixel covered by this output pixel then box downsample
    var value = 0.0;
    for (var sy = 0; sy < supersample; sy++) {
        for (var sx = 0; sx < supersample; sx++) {
            let subpixel = vec2<i32>(sx, sy);
            let scene_pixel = pixel * supersample + subpixel;
            let scene_position = vec2<f32>(scene_pixel) + 0.5;
            let sample = load_scene(scene_pixel);
            var luma = sample.r;
            var object_pattern = sample.g;
            if in_probe(scene_position / f32(supersample)) {
                luma = params.probe_gray;
                object_pattern = 1.0;
            }

            var coord = scene_position;
            switch params.mapping {
                case MAPPING_SCREEN_OFFSET: {
                    coord = scene_position + params.screen_offset * f32(supersample);
                }
                case MAPPING_MOTION_WARP: {
                    coord = scene_position + (params.screen_offset + residual) * f32(supersample);
                }
                case MAPPING_CUBE: {
                    coord = cube_coord(world_direction(scene_position, scene_size), texel_angle);
                }
                case MAPPING_SPHERE: {
                    coord = sphere_coord(world_direction(scene_position, scene_size), texel_angle);
                }
                default: {}
            }
            let threshold = pattern_threshold(
                bayer_texture,
                blue_noise_texture,
                coord,
                use_bayer(object_pattern),
            );

            if params.mapping == MAPPING_TEXEL || params.view_mode == VIEW_SOURCE {
                // texel space thresholding already happened in the scene pass
                value += luma;
            } else if params.view_mode == VIEW_PATTERN {
                value += threshold;
            } else if params.view_mode == VIEW_FLAT_GRAY {
                value += select(0.0, 1.0, 0.5 > threshold);
            } else {
                value += select(0.0, 1.0, luma > threshold);
            }
        }
    }
    value /= f32(supersample * supersample);

    let is_binary_view = params.view_mode == VIEW_OUTPUT || params.view_mode == VIEW_FLAT_GRAY;
    if params.final_threshold != 0u && is_binary_view {
        value = select(0.0, 1.0, value >= 0.5);
    }

    let is_scene_view = params.view_mode == VIEW_OUTPUT || params.view_mode == VIEW_SOURCE;
    let pixel_in_probe = in_probe(vec2<f32>(pixel) + 0.5);
    if params.edges != 0u && is_scene_view && !pixel_in_probe && is_edge(pixel, center.b) {
        value = line_value(pixel);
    }

    return vec4<f32>(value, wrap_centered(residual), depth);
}
