use avian2d::prelude::*;
use bevy::{prelude::*, window::PresentMode};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rand::plugin::EntropyPlugin;
use bevy_rand::prelude::*;

mod terrain;
mod main_menu;
mod game;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum GameState {
    #[default]
    MainMenu,
    InGame,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .init_state::<GameState>()
        .insert_state(GameState::MainMenu)
        .enable_state_scoped_entities::<GameState>()
        .add_plugins(PhysicsPlugins::default())
        // .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        // .add_plugins(ResourceInspectorPlugin::<PlayerMoveConfig>::default())
        // .add_plugins(PanCamPlugin::default())
        .add_plugins(TilemapPlugin)
        // .add_plugins(terrain::TerrainPlugin)
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .add_plugins(main_menu::MainMenu)
        .add_plugins(game::Game)
        .add_systems(Startup, startup)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

