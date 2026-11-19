//! Procedural textures: the two dither patterns from the devlog (8x8 bayer matrix and
//! 128x128 blue noise field) and a grime texture for the cabin walls.

use bevy::prelude::*;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub const BAYER_SIZE: usize = 8;
pub const BLUE_NOISE_SIZE: usize = 128;
const GRIME_SIZE: usize = 256;
const BLUE_NOISE_SIGMA: f32 = 1.5;
const BLUE_NOISE_RADIUS: i32 = 6;
const SEED: u32 = 42;

//////////////////////////////////////////////////////////////////////

#[derive(Resource, Clone)]
pub struct DitherPatterns {
    pub bayer: Handle<Image>,
    pub blue_noise: Handle<Image>,
    pub grime: Handle<Image>,
}

pub struct PatternPlugin;

impl Plugin for PatternPlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        info!("** generate patterns **");

        let bayer = make_threshold_image(BAYER_SIZE, &make_bayer_ranks(BAYER_SIZE));
        let blue_noise = make_threshold_image(
            BLUE_NOISE_SIZE,
            &make_blue_noise_ranks(BLUE_NOISE_SIZE, SEED),
        );
        let grime = make_grime_image(GRIME_SIZE, SEED);

        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let patterns = DitherPatterns {
            bayer: images.add(bayer),
            blue_noise: images.add(blue_noise),
            grime: images.add(grime),
        };
        app.insert_resource(patterns);
    }
}

//////////////////////////////////////////////////////////////////////

fn hash(value: u32) -> u32 {
    let mut state = value ^ 2747636419;
    state = state.wrapping_mul(2654435769);
    state ^= state >> 16;
    state = state.wrapping_mul(2654435769);
    state ^= state >> 16;
    state.wrapping_mul(2654435769)
}

fn random_float(value: u32) -> f32 {
    hash(value) as f32 / u32::MAX as f32
}

/// Thresholds are stored as 8-bit values, the shader maps byte `b` to `(b + 0.5) / 256`.
fn make_threshold_image(size: usize, ranks: &[u32]) -> Image {
    let num_ranks = (size * size) as u64;
    let data = ranks
        .iter()
        .map(|&rank| ((2 * rank as u64 + 1) * 256 / (2 * num_ranks)) as u8)
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });
    image
}

/// Classic recursive ordered dither matrix.
fn make_bayer_ranks(size: usize) -> Vec<u32> {
    assert!(size.is_power_of_two());
    let mut ranks = vec![0u32];
    let mut current = 1;
    while current < size {
        let next = current * 2;
        let mut next_ranks = vec![0u32; next * next];
        for yy in 0..current {
            for xx in 0..current {
                let rank = 4 * ranks[yy * current + xx];
                next_ranks[yy * next + xx] = rank;
                next_ranks[yy * next + xx + current] = rank + 2;
                next_ranks[(yy + current) * next + xx] = rank + 3;
                next_ranks[(yy + current) * next + xx + current] = rank + 1;
            }
        }
        ranks = next_ranks;
        current = next;
    }
    ranks
}

/// Ulichney's void-and-cluster method on a torus.
fn make_blue_noise_ranks(size: usize, seed: u32) -> Vec<u32> {
    let num_pixels = size * size;
    let kernel_width = (2 * BLUE_NOISE_RADIUS + 1) as usize;
    let mut kernel = Vec::with_capacity(kernel_width * kernel_width);
    for dy in -BLUE_NOISE_RADIUS..=BLUE_NOISE_RADIUS {
        for dx in -BLUE_NOISE_RADIUS..=BLUE_NOISE_RADIUS {
            let dist2 = (dx * dx + dy * dy) as f32;
            kernel.push((-dist2 / (2.0 * BLUE_NOISE_SIGMA * BLUE_NOISE_SIGMA)).exp());
        }
    }

    let splat = |energy: &mut [f32], index: usize, sign: f32| {
        let x0 = (index % size) as i32;
        let y0 = (index / size) as i32;
        let mut kk = 0;
        for dy in -BLUE_NOISE_RADIUS..=BLUE_NOISE_RADIUS {
            let yy = (y0 + dy).rem_euclid(size as i32) as usize;
            for dx in -BLUE_NOISE_RADIUS..=BLUE_NOISE_RADIUS {
                let xx = (x0 + dx).rem_euclid(size as i32) as usize;
                energy[yy * size + xx] += sign * kernel[kk];
                kk += 1;
            }
        }
    };
    let tightest_cluster = |pattern: &[bool], energy: &[f32]| -> usize {
        let mut best = (f32::MIN, 0);
        for (index, (&set, &value)) in pattern.iter().zip(energy).enumerate() {
            if set && value > best.0 {
                best = (value, index);
            }
        }
        best.1
    };
    let largest_void = |pattern: &[bool], energy: &[f32]| -> usize {
        let mut best = (f32::MAX, 0);
        for (index, (&set, &value)) in pattern.iter().zip(energy).enumerate() {
            if !set && value < best.0 {
                best = (value, index);
            }
        }
        best.1
    };

    // initial random binary pattern
    let num_initial = num_pixels / 10;
    let mut pattern = vec![false; num_pixels];
    let mut energy = vec![0.0f32; num_pixels];
    let mut count = 0;
    let mut counter = seed;
    while count < num_initial {
        counter = counter.wrapping_add(1);
        let index = hash(counter) as usize % num_pixels;
        if !pattern[index] {
            pattern[index] = true;
            splat(&mut energy, index, 1.0);
            count += 1;
        }
    }

    // relax it by moving the tightest cluster to the largest void until stable
    for _ in 0..num_pixels {
        let cluster = tightest_cluster(&pattern, &energy);
        pattern[cluster] = false;
        splat(&mut energy, cluster, -1.0);
        let void = largest_void(&pattern, &energy);
        pattern[void] = true;
        splat(&mut energy, void, 1.0);
        if void == cluster {
            break;
        }
    }

    let mut ranks = vec![0u32; num_pixels];

    // phase 1: rank the initial points by removing tightest clusters
    {
        let mut pattern = pattern.clone();
        let mut energy = energy.clone();
        let mut ones = num_initial;
        while ones > 0 {
            let cluster = tightest_cluster(&pattern, &energy);
            pattern[cluster] = false;
            splat(&mut energy, cluster, -1.0);
            ones -= 1;
            ranks[cluster] = ones as u32;
        }
    }

    // phase 2 & 3: fill the largest voids until the pattern is full
    let mut ones = num_initial;
    while ones < num_pixels {
        let void = largest_void(&pattern, &energy);
        pattern[void] = true;
        splat(&mut energy, void, 1.0);
        ranks[void] = ones as u32;
        ones += 1;
    }

    ranks
}

//////////////////////////////////////////////////////////////////////

/// Tileable value noise.
fn value_noise(xx: f32, yy: f32, period: UVec2, seed: u32) -> f32 {
    let lattice = |ix: i32, iy: i32| -> f32 {
        let ix = ix.rem_euclid(period.x as i32) as u32;
        let iy = iy.rem_euclid(period.y as i32) as u32;
        random_float(seed ^ hash(ix.wrapping_mul(7919) ^ iy.wrapping_mul(104729)))
    };
    let x0 = xx.floor();
    let y0 = yy.floor();
    let fx = xx - x0;
    let fy = yy - y0;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let (ix, iy) = (x0 as i32, y0 as i32);
    let aa = lattice(ix, iy);
    let bb = lattice(ix + 1, iy);
    let cc = lattice(ix, iy + 1);
    let dd = lattice(ix + 1, iy + 1);
    let top = aa + (bb - aa) * sx;
    let bottom = cc + (dd - cc) * sx;
    top + (bottom - top) * sy
}

/// Vertical streaks of dirt, like the stained walls of the devlog test cabin.
fn make_grime_image(size: usize, seed: u32) -> Image {
    let mut data = Vec::with_capacity(size * size * 4);
    for yy in 0..size {
        for xx in 0..size {
            let uu = xx as f32 / size as f32;
            let vv = yy as f32 / size as f32;
            let mut streaks = 0.0;
            let mut amplitude = 0.5;
            let mut frequency = uvec2(40, 2);
            for octave in 0..4 {
                streaks += amplitude
                    * value_noise(
                        uu * frequency.x as f32,
                        vv * frequency.y as f32,
                        frequency,
                        seed.wrapping_add(octave),
                    );
                amplitude *= 0.5;
                frequency *= 2;
            }
            let blotches = value_noise(uu * 8.0, vv * 8.0, uvec2(8, 8), seed.wrapping_add(17));
            let dirt = (streaks - 0.5).max(0.0) * 1.8 + (blotches - 0.65).max(0.0) * 0.6;
            let value = (0.92 - dirt).clamp(0.35, 1.0);
            let byte = (value * 255.0) as u8;
            data.extend_from_slice(&[byte, byte, byte, 255]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}
