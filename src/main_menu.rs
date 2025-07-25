use bevy::prelude::*;

use crate::GameState;

pub struct MainMenu;

impl Plugin for MainMenu {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), on_enter)
            .add_systems(
                Update,
                check_for_any_key.run_if(in_state(GameState::MainMenu)),
            );
    }
}

fn on_enter(mut commands: Commands) {
    commands.spawn((
        StateScoped(GameState::MainMenu),
        Text::new("Press any key to start"),
    ));
}

fn check_for_any_key(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.get_just_pressed().len() > 0 {
        commands.set_state(GameState::InGame);
    }
}
