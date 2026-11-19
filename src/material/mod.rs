pub mod display_material;
pub mod dither_material;
pub mod scene_material;

use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<scene_material::SceneMaterial>::default());
        app.add_plugins(Material2dPlugin::<dither_material::DitherMaterial>::default());
        app.add_plugins(Material2dPlugin::<display_material::DisplayMaterial>::default());
    }
}
