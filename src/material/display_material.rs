//! Display pass: upscales the low resolution output to the window and applies the palette.

use bevy::prelude::*;

use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::Material2d;

const SHADER_PATH: &str = "shaders/display.wesl";

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct DisplayParams {
    pub color_dark: Vec4,
    pub color_light: Vec4,
    pub source_size: Vec2,
    /// Physical window pixels per source pixel.
    pub texel_scale: f32,
    /// Sharp bilinear filtering instead of nearest neighbor.
    pub sharp_bilinear: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DisplayMaterial {
    #[uniform(0)]
    pub params: DisplayParams,
    #[texture(1)]
    pub source_texture: Handle<Image>,
}

impl Material2d for DisplayMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}
