use bevy::prelude::*;

use crate::{GameState, toggle_world_inspector};

pub struct MainMenu;

impl Plugin for MainMenu {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), on_enter)
            .add_systems(
                Update,
                check_for_any_key
                    .run_if(in_state(GameState::MainMenu))
                    .after(toggle_world_inspector),
            );
    }
}

fn on_enter(mut commands: Commands) {
    commands.spawn((
        StateScoped(GameState::MainMenu),
        Text2d::new("Press any key to start"),
        TextLayout::new_with_justify(JustifyText::Center),
    ));
}

fn check_for_any_key(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.get_just_released().next().is_some() {
        commands.set_state(GameState::SpaceShooter);
    }
}
