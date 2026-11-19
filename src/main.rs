//! dithering ftw: Return of the Obra Dinn style 1-bit rendering, after Lucas Pope's devlog
//! <https://dukope.com/devlogs/obra-dinn/tig-32/>

mod camera;
mod dither;
mod global_state;
mod material;
mod patterns;
mod scene;
mod ui;

use bevy::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "dithering ftw".into(),
            canvas: Some("#mygame-canvas".into()),
            resolution: (1600, 900).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(global_state::GlobalStatePlugin);
    app.add_plugins(patterns::PatternPlugin);
    app.add_plugins(material::CustomMaterialPlugin);
    app.add_plugins(camera::CameraPlugin);
    app.add_plugins(dither::DitherPlugin);
    app.add_plugins(scene::ScenePlugin);
    app.add_plugins(ui::UiPlugin);

    #[cfg(not(target_family = "wasm"))]
    {
        app.add_systems(Update, keyboard_shortcuts);
    }

    app.run();
}

#[cfg(not(target_family = "wasm"))]
fn keyboard_shortcuts(
    mut commands: Commands,
    mut writer: MessageWriter<AppExit>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    if keyboard.just_pressed(KeyCode::Escape) {
        writer.write(AppExit::Success);
    }
    if keyboard.just_pressed(KeyCode::F12) {
        let path = format!("snapshot_{}.png", time.elapsed().as_millis());
        info!("** snapshot {path} **");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}
