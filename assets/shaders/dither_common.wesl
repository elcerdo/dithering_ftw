//! Shared dither pattern helpers.

const BAYER_SIZE: i32 = 8;
const BLUE_NOISE_SIZE: i32 = 128;

/// Positive modulo so that the patterns tile in every direction.
fn wrap_index(index: vec2<i32>, size: i32) -> vec2<i32> {
    let modulo = index % size;
    return select(modulo, modulo + size, modulo < vec2<i32>(0));
}

/// 8-bit dither pattern value at a pattern coordinate, as a threshold in ]0, 1[.
fn pattern_threshold(
    bayer: texture_2d<f32>,
    blue_noise: texture_2d<f32>,
    coord: vec2<f32>,
    use_bayer: bool,
) -> f32 {
    let index = vec2<i32>(floor(coord));
    var raw = 0.0;
    if use_bayer {
        raw = textureLoad(bayer, wrap_index(index, BAYER_SIZE), 0).r;
    } else {
        raw = textureLoad(blue_noise, wrap_index(index, BLUE_NOISE_SIZE), 0).r;
    }
    return (round(raw * 255.0) + 0.5) / 256.0;
}
