use bevy::prelude::*;
use bevy_egui::input::egui_wants_any_keyboard_input;

use crate::{GameState, toggle_world_inspector};

pub struct MainMenu;

impl Plugin for MainMenu {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), on_enter)
            .add_systems(
                Update,
                (
                    check_main_menu_keys.run_if(in_state(GameState::MainMenu)),
                    check_for_exit.run_if(not(in_state(GameState::MainMenu))),
                )
                    .run_if(not(egui_wants_any_keyboard_input))
                    .after(toggle_world_inspector),
            );
    }
}

fn on_enter(mut commands: Commands) {
    commands.spawn((
        StateScoped(GameState::MainMenu),
        Text2d::new("1: Shooter\n2: Conveyor"),
        TextLayout::new_with_justify(JustifyText::Center),
    ));
}

fn check_main_menu_keys(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    match keys.get_just_released().next() {
        Some(KeyCode::Digit1) => commands.set_state(GameState::SpaceShooter),
        Some(KeyCode::Digit2) => commands.set_state(GameState::FactoryGame),
        _ => (),
    }
}

fn check_for_exit(mut commands: Commands, mut keys: ResMut<ButtonInput<KeyCode>>) {
    if keys.clear_just_pressed(KeyCode::Escape) {
        commands.set_state(GameState::MainMenu);
    }
}
