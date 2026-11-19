//! Scene pass material: standard pbr lighting reduced to the 8-bit grayscale "source image"
//! plus the per-object data needed by the post-processing pass.
//!
//! Output channels: r = luminance, g = dither pattern selector, b = region id (wireframe), a = view depth.

use crate::patterns::DitherPatterns;

use bevy::prelude::*;

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

const SHADER_PATH: &str = "shaders/scene.wesl";

pub type SceneMaterial = ExtendedMaterial<StandardMaterial, SceneExtension>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectPattern {
    BlueNoise,
    Bayer,
}

/// Texel space dithering is done while rendering the scene, see `TEXEL_MODE_*` in the shader.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TexelMode {
    Off = 0,
    Threshold = 1,
    Pattern = 2,
    FlatGray = 3,
}

#[derive(Clone, Copy, Debug, Default, Reflect, ShaderType)]
pub struct SceneParams {
    pub region: f32,
    pub pattern: f32,
    pub stripes: Vec2,
    pub texel_scale: f32,
    pub texel_mode: u32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct SceneExtension {
    // bindings 0-99 are reserved for the base StandardMaterial
    #[uniform(100)]
    pub params: SceneParams,
    #[texture(101)]
    pub bayer_texture: Handle<Image>,
    #[texture(102)]
    pub blue_noise_texture: Handle<Image>,
}

impl MaterialExtension for SceneExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

pub struct SceneLook {
    pub gray: f32,
    pub roughness: f32,
    pub pattern: ObjectPattern,
    /// Splits the object in sub regions along uv, e.g. barrel staves.
    pub stripes: Vec2,
    /// Dither pattern texels per uv unit when dithering in texel space.
    pub texel_scale: f32,
    pub texture: Option<(Handle<Image>, Vec2)>,
}

impl Default for SceneLook {
    fn default() -> Self {
        Self {
            gray: 0.7,
            roughness: 0.9,
            pattern: ObjectPattern::BlueNoise,
            stripes: Vec2::ZERO,
            texel_scale: 64.0,
            texture: None,
        }
    }
}

pub fn make(patterns: &DitherPatterns, look: &SceneLook, region: u32) -> SceneMaterial {
    use bevy::math::Affine2;
    let (base_color_texture, uv_transform) = match &look.texture {
        Some((texture, scale)) => (Some(texture.clone()), Affine2::from_scale(*scale)),
        None => (None, Affine2::IDENTITY),
    };
    ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::linear_rgb(look.gray, look.gray, look.gray),
            base_color_texture,
            uv_transform,
            perceptual_roughness: look.roughness,
            reflectance: 0.3,
            metallic: 0.0,
            ..default()
        },
        extension: SceneExtension {
            params: SceneParams {
                region: region as f32,
                pattern: match look.pattern {
                    ObjectPattern::BlueNoise => 0.0,
                    ObjectPattern::Bayer => 1.0,
                },
                stripes: look.stripes,
                texel_scale: look.texel_scale,
                texel_mode: TexelMode::Off as u32,
            },
            bayer_texture: patterns.bayer.clone(),
            blue_noise_texture: patterns.blue_noise.clone(),
        },
    }
}

pub fn set_texel_mode(materials: &mut Assets<SceneMaterial>, texel_mode: TexelMode) {
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if let Some(mut material) = materials.get_mut(id) {
            material.extension.params.texel_mode = texel_mode as u32;
        }
    }
}
