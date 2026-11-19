//! User facing options of the dithering pipeline, mirroring the devlog experiments.

use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mapping {
    Screen = 0,
    ScreenOffset = 1,
    Texel = 2,
    MotionWarp = 3,
    Cube = 4,
    Sphere = 5,
}

pub const MAPPINGS: [(Mapping, &str); 6] = [
    (Mapping::Screen, "map: screen"),
    (Mapping::ScreenOffset, "map: screen+offset"),
    (Mapping::Texel, "map: texel"),
    (Mapping::MotionWarp, "map: motion warp"),
    (Mapping::Cube, "map: cube"),
    (Mapping::Sphere, "map: sphere"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Output = 0,
    Source = 1,
    Pattern = 2,
    FlatGray = 3,
}

pub const VIEW_MODES: [(ViewMode, &str); 4] = [
    (ViewMode::Output, "view: 1-bit output"),
    (ViewMode::Source, "view: 8-bit source"),
    (ViewMode::Pattern, "view: dither pattern"),
    (ViewMode::FlatGray, "view: flat gray"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatternMode {
    PerObject = 0,
    Bayer = 1,
    BlueNoise = 2,
}

pub const PATTERN_MODES: [(PatternMode, &str); 3] = [
    (PatternMode::PerObject, "pattern: per object"),
    (PatternMode::Bayer, "pattern: bayer"),
    (PatternMode::BlueNoise, "pattern: blue noise"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fit {
    /// Border boxed, integer upscaling, crisp pixels.
    Digital,
    /// Fills the window, sharp bilinear upscaling.
    Analog,
}

pub const FITS: [(Fit, &str); 2] = [
    (Fit::Digital, "display: digital"),
    (Fit::Analog, "display: analog"),
];

/// (name, dark, light) in sRGB.
pub const PALETTES: [(&str, [u8; 3], [u8; 3]); 4] = [
    ("palette: obra dinn", [0x33, 0x33, 0x19], [0xe5, 0xff, 0xff]),
    ("palette: 1-bit", [0x00, 0x00, 0x00], [0xff, 0xff, 0xff]),
    ("palette: amber", [0x28, 0x18, 0x08], [0xff, 0xb8, 0x40]),
    ("palette: green", [0x10, 0x20, 0x14], [0x40, 0xff, 0x80]),
];

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct DitherSettings {
    pub mapping: Mapping,
    pub view: ViewMode,
    pub pattern: PatternMode,
    pub supersample: bool,
    pub final_threshold: bool,
    pub edges: bool,
    pub palette: usize,
    pub fit: Fit,
    pub probe: bool,
}

impl DitherSettings {
    /// Shipped "DIGITAL" mode: border-boxed, screenspace offset dither, 1-bit output.
    pub fn digital() -> Self {
        Self {
            mapping: Mapping::ScreenOffset,
            view: ViewMode::Output,
            pattern: PatternMode::PerObject,
            supersample: false,
            final_threshold: true,
            edges: true,
            palette: 0,
            fit: Fit::Digital,
            probe: true,
        }
    }

    /// Shipped "ANALOG" mode: fullscreen, sphere-mapped dither, softened output.
    pub fn analog() -> Self {
        Self {
            mapping: Mapping::Sphere,
            view: ViewMode::Output,
            pattern: PatternMode::PerObject,
            supersample: true,
            final_threshold: false,
            edges: true,
            palette: 0,
            fit: Fit::Analog,
            probe: true,
        }
    }

    pub fn palette_colors(&self) -> (Srgba, Srgba) {
        let (_, dark, light) = PALETTES[self.palette % PALETTES.len()];
        (
            Srgba::rgb_u8(dark[0], dark[1], dark[2]),
            Srgba::rgb_u8(light[0], light[1], light[2]),
        )
    }
}

impl Default for DitherSettings {
    fn default() -> Self {
        Self::digital()
    }
}
