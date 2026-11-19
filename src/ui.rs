//! Settings panel (egui) and keyboard shortcuts. Both edit the settings resources directly.

use crate::camera::CameraSettings;
use crate::dither::OUTPUT_SIZE;
use crate::dither::settings::*;
use crate::scene::SceneSettings;

use bevy::prelude::*;

use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiInputSystemSettings, EguiPlugin, EguiPrimaryContextPass,
    egui,
};

const PANEL_MARGIN: f32 = 8.0;
const PANEL_WIDTH: f32 = 210.0;
const MAPPING_KEYS: [KeyCode; 6] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
];
const CONTROLS: [(&str, &str); 14] = [
    ("1-6", "mapping"),
    ("V", "view"),
    ("B", "pattern"),
    ("P", "palette"),
    ("X", "2x supersample"),
    ("T", "final threshold"),
    ("E", "edges"),
    ("C", "pattern probe"),
    ("RMB drag", "look"),
    ("arrows", "look"),
    ("WASD", "walk"),
    ("space", "auto pan"),
    ("M", "move sphere"),
    ("H F12", "hide panel / snapshot"),
];

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default());
        app.insert_resource(EguiGlobalSettings {
            // the primary context goes on the display camera, see `dither::populate_pipeline`
            auto_create_primary_context: false,
            // the panel has no text field, keys stay with the shortcuts
            input_system_settings: EguiInputSystemSettings {
                run_write_keyboard_input_messages_system: false,
                ..default()
            },
            ..default()
        });
        app.init_resource::<UiState>();
        app.add_systems(Update, keyboard_shortcuts);
        app.add_systems(EguiPrimaryContextPass, settings_panel);
    }
}

#[derive(Resource)]
pub struct UiState {
    visible: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self { visible: true }
    }
}

//////////////////////////////////////////////////////////////////////

fn next<T: Copy + PartialEq>(items: &[(T, &str)], value: T) -> T {
    let index = items.iter().position(|(item, _)| *item == value).unwrap();
    items[(index + 1) % items.len()].0
}

pub fn keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DitherSettings>,
    mut camera_settings: ResMut<CameraSettings>,
    mut scene_settings: ResMut<SceneSettings>,
    mut ui_state: ResMut<UiState>,
) {
    let mut new_settings = settings.clone();
    for (key, (mapping, _)) in MAPPING_KEYS.iter().zip(MAPPINGS) {
        if keyboard.just_pressed(*key) {
            new_settings.mapping = mapping;
        }
    }
    if keyboard.just_pressed(KeyCode::KeyV) {
        new_settings.view = next(&VIEW_MODES, new_settings.view);
    }
    if keyboard.just_pressed(KeyCode::KeyB) {
        new_settings.pattern = next(&PATTERN_MODES, new_settings.pattern);
    }
    if keyboard.just_pressed(KeyCode::KeyP) {
        new_settings.palette = (new_settings.palette + 1) % PALETTES.len();
    }
    let toggles = [
        (KeyCode::KeyX, &mut new_settings.supersample),
        (KeyCode::KeyT, &mut new_settings.final_threshold),
        (KeyCode::KeyE, &mut new_settings.edges),
        (KeyCode::KeyC, &mut new_settings.probe),
    ];
    for (key, value) in toggles {
        if keyboard.just_pressed(key) {
            *value ^= true;
        }
    }
    settings.set_if_neq(new_settings);

    if keyboard.just_pressed(KeyCode::Space) {
        camera_settings.auto_pan ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyM) {
        scene_settings.move_sphere ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyH) {
        ui_state.visible ^= true;
    }
}

//////////////////////////////////////////////////////////////////////

fn combo<T: Copy + PartialEq>(ui: &mut egui::Ui, label: &str, value: &mut T, items: &[(T, &str)]) {
    let selected = items
        .iter()
        .find(|(item, _)| item == value)
        .map_or("", |(_, name)| *name);
    egui::ComboBox::from_label(label)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (item, name) in items {
                ui.selectable_value(value, *item, *name);
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn settings_panel(
    mut contexts: EguiContexts,
    time: Res<Time>,
    ui_state: Res<UiState>,
    mut settings: ResMut<DitherSettings>,
    mut camera_settings: ResMut<CameraSettings>,
    mut scene_settings: ResMut<SceneSettings>,
    mut smoothed_dt: Local<f32>,
    mut is_styled: Local<bool>,
) -> Result {
    *smoothed_dt = 0.95 * *smoothed_dt + 0.05 * time.delta_secs();

    let ctx = contexts.ctx_mut()?;
    if !*is_styled {
        ctx.set_visuals(egui::Visuals::light());
        *is_styled = true;
    }
    if !ui_state.visible {
        return Ok(());
    }

    let mut new_settings = settings.clone();
    let mut auto_pan = camera_settings.auto_pan;
    let mut move_sphere = scene_settings.move_sphere;
    let palettes: Vec<(usize, &str)> = PALETTES
        .iter()
        .enumerate()
        .map(|(index, (name, _, _))| (index, *name))
        .collect();

    egui::Window::new("dithering ftw")
        .anchor(egui::Align2::RIGHT_TOP, [-PANEL_MARGIN, PANEL_MARGIN])
        .default_width(PANEL_WIDTH)
        .resizable(false)
        .show(ctx, |ui| {
            let fps = if *smoothed_dt > 0.0 {
                1.0 / *smoothed_dt
            } else {
                0.0
            };
            let scene_size = OUTPUT_SIZE * if new_settings.supersample { 2 } else { 1 };
            ui.monospace(format!("{fps:.0} fps"));
            ui.monospace(format!(
                "{}x{} -> {}x{}",
                scene_size.x, scene_size.y, OUTPUT_SIZE.x, OUTPUT_SIZE.y
            ));
            ui.separator();

            combo(ui, "mapping", &mut new_settings.mapping, &MAPPINGS);
            combo(ui, "view", &mut new_settings.view, &VIEW_MODES);
            combo(ui, "pattern", &mut new_settings.pattern, &PATTERN_MODES);
            combo(ui, "palette", &mut new_settings.palette, &palettes);
            ui.checkbox(&mut new_settings.supersample, "2x supersample");
            ui.checkbox(&mut new_settings.final_threshold, "final threshold");
            ui.checkbox(&mut new_settings.edges, "edges");
            ui.checkbox(&mut new_settings.probe, "pattern probe");
            ui.separator();

            ui.checkbox(&mut auto_pan, "auto pan");
            ui.checkbox(&mut move_sphere, "move sphere");

            ui.collapsing("controls", |ui| {
                egui::Grid::new("controls").striped(true).show(ui, |ui| {
                    for (keys, action) in CONTROLS {
                        ui.monospace(keys);
                        ui.label(action);
                        ui.end_row();
                    }
                });
            });
        });

    settings.set_if_neq(new_settings);
    if camera_settings.auto_pan != auto_pan {
        camera_settings.auto_pan = auto_pan;
    }
    if scene_settings.move_sphere != move_sphere {
        scene_settings.move_sphere = move_sphere;
    }
    Ok(())
}
