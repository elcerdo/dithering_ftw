use bevy::prelude::*;

use bevy::ui::{Pressed, RelativeCursorPosition};
use bevy::ui_widgets::Button;

use super::colors::*;

#[derive(Component)]
pub struct UiCheckbox {
    label: String,
    pub checked: bool,
}

pub fn make(frame: &mut EntityCommands<'_>, label: &str, checked: bool) -> Entity {
    let mut ret = Option::None;
    frame.with_children(|parent| {
        let node = make_default_node();
        let mut container = parent.spawn((
            UiCheckbox {
                label: label.into(),
                checked,
            },
            Button,
            RelativeCursorPosition::default(),
            node.clone(),
            BorderColor::all(COLOR_UI_FG),
            BackgroundColor(COLOR_UI_BG_DISABLED.into()),
        ));
        container.with_child((
            Text::new(label),
            TextFont::from_font_size(FONT_SIZE),
            TextColor(COLOR_UI_FG.into()),
        ));
        ret = Some(container.id());
    });
    return ret.unwrap();
}

pub fn update(mut checkboxes: Query<&mut UiCheckbox, (Added<Pressed>, With<Button>)>) {
    for mut data in checkboxes.iter_mut() {
        data.checked ^= true;
        info!("***** checkbox {} {}", data.label, data.checked);
    }
}

/// Also picks up changes made from keyboard shortcuts.
pub fn sync(mut checkboxes: Query<(&UiCheckbox, &mut BackgroundColor), Changed<UiCheckbox>>) {
    for (data, mut bg_color) in checkboxes.iter_mut() {
        *bg_color = if data.checked {
            COLOR_UI_BG.into()
        } else {
            COLOR_UI_BG_DISABLED.into()
        };
    }
}
