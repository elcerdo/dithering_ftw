//! Post-processing pass: converts the 8-bit grayscale scene to 1-bit at the output resolution.
//!
//! Output channels: r = output value, gb = motion warp residual, a = view depth.

use bevy::prelude::*;

use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::Material2d;

const SHADER_PATH: &str = "shaders/dither.wesl";

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct DitherParams {
    pub world_from_view: Mat4,
    pub prev_clip_from_world: Mat4,
    pub output_size: Vec2,
    pub tan_half_fov: Vec2,
    /// Screen mapping offset following the camera rotation, in output pixels.
    pub screen_offset: Vec2,
    /// Previous minus current offset, unwrapped.
    pub screen_offset_delta: Vec2,
    pub supersample: u32,
    pub mapping: u32,
    pub view_mode: u32,
    pub pattern_mode: u32,
    pub edges: u32,
    pub final_threshold: u32,
    pub reset: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DitherMaterial {
    #[uniform(0)]
    pub params: DitherParams,
    #[texture(1)]
    pub scene_texture: Handle<Image>,
    #[texture(2)]
    pub bayer_texture: Handle<Image>,
    #[texture(3)]
    pub blue_noise_texture: Handle<Image>,
    #[texture(4)]
    pub previous_texture: Handle<Image>,
}

impl Material2d for DitherMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}
