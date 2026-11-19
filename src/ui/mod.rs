mod checkbox;
mod colors;
mod combobox;
mod labels;

use crate::camera::CameraSettings;
use crate::dither::settings::*;
use crate::scene::SceneSettings;

use bevy::prelude::*;

use checkbox::UiCheckbox;
use combobox::UiCombobox;

const HELP: &str = "\
1-6 mapping   V view
B pattern   P palette
F display   X supersample
T threshold   E edges
C probe   H hide ui
F1 digital   F2 analog
RMB drag/arrows look
WASD walk   space auto pan
M move sphere   F12 snapshot";

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, populate_ui);
        app.add_systems(
            Update,
            (
                keyboard_shortcuts,
                combobox::update,
                checkbox::update,
                combobox::sync,
                checkbox::sync,
                apply_widgets,
                update_status,
            )
                .chain(),
        );
    }
}

#[derive(Resource)]
pub struct UiState {
    root: Entity,
    status_label: Entity,
    mapping: Entity,
    view: Entity,
    pattern: Entity,
    palette: Entity,
    fit: Entity,
    supersample: Entity,
    final_threshold: Entity,
    edges: Entity,
    probe: Entity,
    auto_pan: Entity,
    move_sphere: Entity,
}

fn names<T>(items: &[(T, &'static str)]) -> Vec<&'static str> {
    items.iter().map(|(_, name)| *name).collect()
}

fn index_of<T: PartialEq>(items: &[(T, &str)], value: T) -> usize {
    items.iter().position(|(item, _)| *item == value).unwrap()
}

fn populate_ui(mut commands: Commands, camera_settings: Res<CameraSettings>) {
    let settings = DitherSettings::default();

    let mut ui_frame = commands.spawn(Node {
        position_type: PositionType::Absolute,
        right: Val::Px(5.0),
        top: Val::Px(5.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexEnd,
        justify_content: JustifyContent::FlexEnd,
        ..default()
    });
    let root = ui_frame.id();

    let status_label = labels::make(&mut ui_frame, "status");

    let mapping = combobox::make(
        &mut ui_frame,
        names(&MAPPINGS),
        index_of(&MAPPINGS, settings.mapping),
    );
    let view = combobox::make(
        &mut ui_frame,
        names(&VIEW_MODES),
        index_of(&VIEW_MODES, settings.view),
    );
    let pattern = combobox::make(
        &mut ui_frame,
        names(&PATTERN_MODES),
        index_of(&PATTERN_MODES, settings.pattern),
    );
    let palette = combobox::make(
        &mut ui_frame,
        PALETTES.iter().map(|(name, _, _)| *name).collect(),
        settings.palette,
    );
    let fit = combobox::make(&mut ui_frame, names(&FITS), index_of(&FITS, settings.fit));

    let supersample = checkbox::make(&mut ui_frame, "2x supersample", settings.supersample);
    let final_threshold =
        checkbox::make(&mut ui_frame, "final threshold", settings.final_threshold);
    let edges = checkbox::make(&mut ui_frame, "edges", settings.edges);
    let probe = checkbox::make(&mut ui_frame, "pattern probe", settings.probe);
    let auto_pan = checkbox::make(&mut ui_frame, "auto pan", camera_settings.auto_pan);
    let move_sphere = checkbox::make(&mut ui_frame, "move sphere", false);

    labels::make(&mut ui_frame, HELP);

    commands.insert_resource(UiState {
        root,
        status_label,
        mapping,
        view,
        pattern,
        palette,
        fit,
        supersample,
        final_threshold,
        edges,
        probe,
        auto_pan,
        move_sphere,
    });
}

fn apply_preset(
    ui_state: &UiState,
    preset: &DitherSettings,
    comboboxes: &mut Query<&mut UiCombobox>,
    checkboxes: &mut Query<&mut UiCheckbox>,
) {
    let combobox_values = [
        (ui_state.mapping, index_of(&MAPPINGS, preset.mapping)),
        (ui_state.view, index_of(&VIEW_MODES, preset.view)),
        (ui_state.pattern, index_of(&PATTERN_MODES, preset.pattern)),
        (ui_state.fit, index_of(&FITS, preset.fit)),
    ];
    for (entity, index) in combobox_values {
        comboboxes.get_mut(entity).unwrap().index = index;
    }
    let checkbox_values = [
        (ui_state.supersample, preset.supersample),
        (ui_state.final_threshold, preset.final_threshold),
        (ui_state.edges, preset.edges),
    ];
    for (entity, checked) in checkbox_values {
        checkboxes.get_mut(entity).unwrap().checked = checked;
    }
}

fn keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    mut comboboxes: Query<&mut UiCombobox>,
    mut checkboxes: Query<&mut UiCheckbox>,
    mut visibilities: Query<&mut Visibility>,
) {
    let mapping_keys = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ];
    for (index, key) in mapping_keys.iter().enumerate() {
        if keyboard.just_pressed(*key) {
            comboboxes.get_mut(ui_state.mapping).unwrap().index = index;
        }
    }

    let combobox_keys = [
        (KeyCode::KeyV, ui_state.view),
        (KeyCode::KeyB, ui_state.pattern),
        (KeyCode::KeyP, ui_state.palette),
        (KeyCode::KeyF, ui_state.fit),
    ];
    for (key, entity) in combobox_keys {
        if keyboard.just_pressed(key) {
            let mut combobox = comboboxes.get_mut(entity).unwrap();
            assert!(combobox.len() > 0);
            combobox.cycle();
        }
    }

    let checkbox_keys = [
        (KeyCode::KeyX, ui_state.supersample),
        (KeyCode::KeyT, ui_state.final_threshold),
        (KeyCode::KeyE, ui_state.edges),
        (KeyCode::KeyC, ui_state.probe),
        (KeyCode::Space, ui_state.auto_pan),
        (KeyCode::KeyM, ui_state.move_sphere),
    ];
    for (key, entity) in checkbox_keys {
        if keyboard.just_pressed(key) {
            checkboxes.get_mut(entity).unwrap().checked ^= true;
        }
    }

    if keyboard.just_pressed(KeyCode::F1) {
        apply_preset(
            &ui_state,
            &DitherSettings::digital(),
            &mut comboboxes,
            &mut checkboxes,
        );
    }
    if keyboard.just_pressed(KeyCode::F2) {
        apply_preset(
            &ui_state,
            &DitherSettings::analog(),
            &mut comboboxes,
            &mut checkboxes,
        );
    }

    if keyboard.just_pressed(KeyCode::KeyH) {
        let mut visibility = visibilities.get_mut(ui_state.root).unwrap();
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
    }
}

/// Widgets are the source of truth, keyboard shortcuts edit them too.
pub fn apply_widgets(
    ui_state: Res<UiState>,
    comboboxes: Query<&UiCombobox>,
    checkboxes: Query<&UiCheckbox>,
    mut settings: ResMut<DitherSettings>,
    mut camera_settings: ResMut<CameraSettings>,
    mut scene_settings: ResMut<SceneSettings>,
) {
    let combobox = |entity: Entity| comboboxes.get(entity).unwrap().index;
    let checked = |entity: Entity| checkboxes.get(entity).unwrap().checked;
    let new_settings = DitherSettings {
        mapping: MAPPINGS[combobox(ui_state.mapping)].0,
        view: VIEW_MODES[combobox(ui_state.view)].0,
        pattern: PATTERN_MODES[combobox(ui_state.pattern)].0,
        palette: combobox(ui_state.palette),
        fit: FITS[combobox(ui_state.fit)].0,
        supersample: checked(ui_state.supersample),
        final_threshold: checked(ui_state.final_threshold),
        edges: checked(ui_state.edges),
        probe: checked(ui_state.probe),
    };
    settings.set_if_neq(new_settings);
    camera_settings.auto_pan = checked(ui_state.auto_pan);
    scene_settings.move_sphere = checked(ui_state.move_sphere);
}

fn update_status(
    time: Res<Time>,
    settings: Res<DitherSettings>,
    ui_state: Res<UiState>,
    mut texts: Query<&mut Text>,
    mut smoothed_dt: Local<f32>,
) {
    *smoothed_dt = 0.95 * *smoothed_dt + 0.05 * time.delta_secs();
    let fps = if *smoothed_dt > 0.0 {
        1.0 / *smoothed_dt
    } else {
        0.0
    };
    let factor = if settings.supersample { 2 } else { 1 };
    let size = crate::dither::OUTPUT_SIZE * factor;
    let mut text = texts.get_mut(ui_state.status_label).unwrap();
    **text = format!(
        "{:3.0} fps | {}x{} -> {}x{}",
        fps,
        size.x,
        size.y,
        crate::dither::OUTPUT_SIZE.x,
        crate::dither::OUTPUT_SIZE.y
    );
}
