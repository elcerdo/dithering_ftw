use bevy::prelude::*;

use bevy::ui::{Pressed, RelativeCursorPosition};
use bevy::ui_widgets::Button;

use super::colors::*;

#[derive(Component, Default)]
pub struct UiCombobox {
    names: Vec<String>,
    pub index: usize,
}

impl UiCombobox {
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn cycle(&mut self) {
        self.index = (self.index + 1) % self.names.len();
    }
}

pub fn make(frame: &mut EntityCommands<'_>, names: Vec<&str>, default_index: usize) -> Entity {
    assert!(default_index < names.len());
    let mut ret = Option::None;
    frame.with_children(|parent| {
        let names: Vec<String> = names.into_iter().map(|aa| aa.into()).collect();
        let default_name = names[default_index].clone();
        let node = make_default_node();
        let mut container = parent.spawn((
            UiCombobox {
                names: names.clone(),
                index: default_index,
            },
            Button,
            RelativeCursorPosition::default(),
            node.clone(),
            BorderColor::all(COLOR_UI_FG),
            BackgroundColor(COLOR_UI_BG.into()),
        ));
        let font = TextFont::from_font_size(FONT_SIZE);
        let color = TextColor(COLOR_UI_FG.into());
        container.with_child((Text::new("<"), font.clone(), color));
        container.with_child((Text::new(default_name), font.clone(), color));
        container.with_child((Text::new(">"), font, color));
        ret = Some(container.id());
    });
    return ret.unwrap();
}

pub fn update(
    mut buttons: Query<(&mut UiCombobox, &RelativeCursorPosition), (Added<Pressed>, With<Button>)>,
) {
    for (mut data, relative_cursor) in buttons.iter_mut() {
        let delta = match relative_cursor.normalized {
            None => 1,
            Some(pos) => {
                if pos.x < 0.0 {
                    data.names.len() - 1
                } else {
                    1
                }
            }
        };
        data.index += delta;
        data.index %= data.names.len();
        info!("***** combobox {delta} {}", data.names[data.index]);
    }
}

/// Also picks up changes made from keyboard shortcuts.
pub fn sync(
    buttons: Query<(&UiCombobox, &Children), Changed<UiCombobox>>,
    mut texts: Query<&mut Text>,
) {
    for (data, children) in buttons.iter() {
        assert!(children.len() == 3);
        let label = children[1];
        let mut text = texts.get_mut(label).unwrap();
        **text = data.names[data.index].clone();
    }
}
