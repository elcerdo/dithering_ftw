//! Render pipeline, mirroring Obra Dinn's: the scene is rendered in 8-bit grayscale to an offscreen
//! image, converted to 1-bit at 800x450 in a post-processing pass, then upscaled to the window.
//!
//! scene camera -> scene image (1x or 2x) -> dither camera -> dither image (ping pong) -> display camera

pub mod settings;

use crate::camera::{self, FirstPersonCamera};
use crate::material::display_material::{DisplayMaterial, DisplayParams};
use crate::material::dither_material::{DitherMaterial, DitherParams};
use crate::material::scene_material::{self, SceneMaterial, TexelMode};
use crate::patterns::DitherPatterns;
use crate::ui;

use settings::*;

use bevy::prelude::*;

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Exposure, Hdr, RenderTarget};
use bevy::image::ImageSampler;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::{DebandDither, Msaa, Tonemapping};
use bevy::ui::IsDefaultUiCamera;
use bevy::window::PrimaryWindow;

use std::f32::consts::{PI, TAU};

/// Final 1-bit resolution, as in the devlog.
pub const OUTPUT_SIZE: UVec2 = UVec2::new(800, 450);
pub const FOV_Y: f32 = 60.0 * PI / 180.0;
const CAMERA_NEAR: f32 = 0.05;
const EXPOSURE_EV100: f32 = 7.0;
const LAYER_DITHER: usize = 1;
const LAYER_DISPLAY: usize = 2;

//////////////////////////////////////////////////////////////////////

pub struct DitherPlugin;

impl Plugin for DitherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DitherSettings>();
        app.add_systems(Startup, populate_pipeline);
        app.add_systems(
            Update,
            (update_dither_pass, update_display_pass, update_texel_mode)
                .after(camera::update_camera)
                .after(ui::apply_widgets),
        );
    }
}

//////////////////////////////////////////////////////////////////////

#[derive(Component)]
pub struct SceneCamera;

#[derive(Component)]
struct DitherCamera;

#[derive(Component)]
struct DisplayCamera;

#[derive(Component)]
struct DisplayQuad;

#[derive(Resource)]
struct DitherPipeline {
    /// Indexed by supersampling factor - 1.
    scene_images: [Handle<Image>; 2],
    /// Ping pong targets, the previous frame's output feeds motion warping.
    dither_images: [Handle<Image>; 2],
    dither_material: Handle<DitherMaterial>,
    display_material: Handle<DisplayMaterial>,
    current: usize,
}

/// Camera state carried over from the previous frame.
#[derive(Default)]
struct CameraTracking {
    prev_clip_from_world: Option<Mat4>,
    prev_yaw: Option<f32>,
    unwrapped_yaw: f32,
    prev_screen_offset: Option<Vec2>,
    prev_settings: Option<(Mapping, bool)>,
}

fn make_target_image(size: UVec2) -> Image {
    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba16Float, None);
    image.sampler = ImageSampler::nearest();
    image
}

fn populate_pipeline(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut dither_materials: ResMut<Assets<DitherMaterial>>,
    mut display_materials: ResMut<Assets<DisplayMaterial>>,
    patterns: Res<DitherPatterns>,
) {
    info!("** populate_pipeline **");

    let scene_images = [
        images.add(make_target_image(OUTPUT_SIZE)),
        images.add(make_target_image(OUTPUT_SIZE * 2)),
    ];
    let dither_images = [
        images.add(make_target_image(OUTPUT_SIZE)),
        images.add(make_target_image(OUTPUT_SIZE)),
    ];

    // scene pass: 8-bit grayscale + per object data, no tonemapping nor msaa to keep values exact
    commands.spawn((
        SceneCamera,
        FirstPersonCamera::default(),
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Image(scene_images[0].clone().into()),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_Y,
            near: CAMERA_NEAR,
            ..default()
        }),
        Exposure {
            ev100: EXPOSURE_EV100,
        },
        Hdr,
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
    ));

    // dither pass: fullscreen quad rendered at the output resolution
    let dither_material = dither_materials.add(DitherMaterial {
        params: DitherParams::default(),
        scene_texture: scene_images[0].clone(),
        bayer_texture: patterns.bayer.clone(),
        blue_noise_texture: patterns.blue_noise.clone(),
        previous_texture: dither_images[1].clone(),
    });
    commands.spawn((
        DitherCamera,
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Image(dither_images[0].clone().into()),
        RenderLayers::layer(LAYER_DITHER),
        Hdr,
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
    ));
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(OUTPUT_SIZE.x as f32, OUTPUT_SIZE.y as f32))),
        MeshMaterial2d(dither_material.clone()),
        RenderLayers::layer(LAYER_DITHER),
    ));

    // display pass: upscaled quad + palette in the window, also hosts the ui
    let display_material = display_materials.add(DisplayMaterial {
        params: DisplayParams::default(),
        source_texture: dither_images[0].clone(),
    });
    commands.spawn((
        DisplayCamera,
        IsDefaultUiCamera,
        Camera2d,
        Camera {
            order: 2,
            ..default()
        },
        RenderLayers::layer(LAYER_DISPLAY),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
    ));
    commands.spawn((
        DisplayQuad,
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(display_material.clone()),
        Transform::default(),
        RenderLayers::layer(LAYER_DISPLAY),
    ));

    commands.insert_resource(DitherPipeline {
        scene_images,
        dither_images,
        dither_material,
        display_material,
        current: 0,
    });
}

//////////////////////////////////////////////////////////////////////

fn wrap_angle(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

fn update_dither_pass(
    settings: Res<DitherSettings>,
    mut pipeline: ResMut<DitherPipeline>,
    mut tracking: Local<CameraTracking>,
    mut scene_cameras: Query<(&Transform, &Projection, &mut RenderTarget), With<SceneCamera>>,
    mut dither_cameras: Query<&mut RenderTarget, (With<DitherCamera>, Without<SceneCamera>)>,
    mut materials: ResMut<Assets<DitherMaterial>>,
) {
    let Ok((transform, projection, mut scene_target)) = scene_cameras.single_mut() else {
        return;
    };
    let Projection::Perspective(perspective) = projection else {
        return;
    };

    // ping pong
    pipeline.current = 1 - pipeline.current;
    let current_image = pipeline.dither_images[pipeline.current].clone();
    let previous_image = pipeline.dither_images[1 - pipeline.current].clone();
    for mut target in &mut dither_cameras {
        *target = RenderTarget::Image(current_image.clone().into());
    }
    let scene_image = pipeline.scene_images[settings.supersample as usize].clone();
    if scene_target.as_image() != Some(&scene_image) {
        *scene_target = RenderTarget::Image(scene_image.clone().into());
    }

    let output_size = OUTPUT_SIZE.as_vec2();
    let aspect = output_size.x / output_size.y;
    let tan_half_fov = vec2(aspect, 1.0) * (perspective.fov / 2.0).tan();
    // pixels per radian at the center of the screen
    let center_density = output_size.y / (2.0 * tan_half_fov.y);

    let world_from_view = transform.to_matrix();
    let clip_from_view =
        bevy::math::proj::perspective_infinite_reverse(perspective.fov, aspect, perspective.near);
    let clip_from_world = clip_from_view * world_from_view.inverse();

    // DitherOffset = ScreenSize * CameraRotation / CameraFov, so that the pattern shifts by one
    // screen per fov of rotation. Using the pixel density at the center keeps it exact there.
    let forward = transform.forward();
    let yaw = (-forward.x).atan2(-forward.z);
    let pitch = forward.y.clamp(-1.0, 1.0).asin();
    tracking.unwrapped_yaw += match tracking.prev_yaw {
        Some(prev_yaw) => wrap_angle(yaw - prev_yaw),
        None => yaw,
    };
    tracking.prev_yaw = Some(yaw);
    let screen_offset = -vec2(tracking.unwrapped_yaw, pitch) * center_density;
    let screen_offset_delta = tracking.prev_screen_offset.unwrap_or(screen_offset) - screen_offset;
    tracking.prev_screen_offset = Some(screen_offset);

    let current_settings = (settings.mapping, settings.supersample);
    let reset =
        tracking.prev_clip_from_world.is_none() || tracking.prev_settings != Some(current_settings);
    let prev_clip_from_world = tracking.prev_clip_from_world.unwrap_or(clip_from_world);
    tracking.prev_clip_from_world = Some(clip_from_world);
    tracking.prev_settings = Some(current_settings);

    if let Some(mut material) = materials.get_mut(&pipeline.dither_material) {
        material.params = DitherParams {
            world_from_view,
            prev_clip_from_world,
            output_size,
            tan_half_fov,
            // the patterns tile every 128 pixels, wrap to keep float precision
            screen_offset: screen_offset - 128.0 * (screen_offset / 128.0).floor(),
            screen_offset_delta,
            supersample: if settings.supersample { 2 } else { 1 },
            mapping: settings.mapping as u32,
            view_mode: settings.view as u32,
            pattern_mode: settings.pattern as u32,
            edges: settings.edges as u32,
            final_threshold: settings.final_threshold as u32,
            reset: reset as u32,
        };
        material.scene_texture = scene_image;
        material.previous_texture = previous_image;
    }
}

fn update_display_pass(
    settings: Res<DitherSettings>,
    pipeline: Res<DitherPipeline>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut quads: Query<&mut Transform, With<DisplayQuad>>,
    mut cameras: Query<&mut Camera, With<DisplayCamera>>,
    mut materials: ResMut<Assets<DisplayMaterial>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let physical_size = window.physical_size().as_vec2();
    let output_size = OUTPUT_SIZE.as_vec2();
    let fit_scale = (physical_size / output_size).min_element();
    let scale = match settings.fit {
        Fit::Digital => fit_scale.floor().max(1.0),
        Fit::Analog => fit_scale,
    };

    // align the quad corners to physical pixels
    let quad_size = output_size * scale;
    let corner = (physical_size - quad_size) / 2.0;
    let snap = corner - corner.floor();
    let scale_factor = window.scale_factor();
    for mut transform in &mut quads {
        transform.translation = (vec2(-snap.x, snap.y) / scale_factor).extend(0.0);
        transform.scale = (quad_size / scale_factor).extend(1.0);
    }

    let (dark, light) = settings.palette_colors();
    for mut camera in &mut cameras {
        camera.clear_color = ClearColorConfig::Custom(dark.into());
    }

    if let Some(mut material) = materials.get_mut(&pipeline.display_material) {
        material.params = DisplayParams {
            color_dark: LinearRgba::from(dark).to_vec4(),
            color_light: LinearRgba::from(light).to_vec4(),
            source_size: output_size,
            texel_scale: scale,
            sharp_bilinear: (settings.fit == Fit::Analog) as u32,
        };
        material.source_texture = pipeline.dither_images[pipeline.current].clone();
    }
}

fn update_texel_mode(
    settings: Res<DitherSettings>,
    mut materials: ResMut<Assets<SceneMaterial>>,
    mut last_mode: Local<Option<TexelMode>>,
    added: Query<(), Added<MeshMaterial3d<SceneMaterial>>>,
) {
    let mode = match (settings.mapping, settings.view) {
        (Mapping::Texel, ViewMode::Output) => TexelMode::Threshold,
        (Mapping::Texel, ViewMode::Pattern) => TexelMode::Pattern,
        (Mapping::Texel, ViewMode::FlatGray) => TexelMode::FlatGray,
        _ => TexelMode::Off,
    };
    if *last_mode == Some(mode) && added.is_empty() {
        return;
    }
    scene_material::set_texel_mode(&mut materials, mode);
    *last_mode = Some(mode);
}
