use avian2d::prelude::*;
use bevy::{prelude::*, window::PresentMode};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::{EguiPlugin, PrimaryEguiContext};
use bevy_enhanced_input::EnhancedInputPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pancam::PanCamPlugin;
use bevy_rand::plugin::EntropyPlugin;
use bevy_rand::prelude::*;

use crate::sprite_sheet::SpriteSheet;

mod factory_game;
mod helpers;
mod main_menu;
mod space_shooter;
mod sprite_sheet;
// mod terrain;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum GameState {
    #[default]
    MainMenu,
    SpaceShooter,
    FactoryGame,
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
        .add_plugins(bevy_mod_debugdump::CommandLineArgs)
        .init_resource::<SpriteSheet>()
        .add_plugins(EnhancedInputPlugin)
        .init_state::<GameState>()
        .insert_state(GameState::MainMenu)
        .enable_state_scoped_entities::<GameState>()
        .add_plugins(PhysicsPlugins::default())
        // .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new().run_if(resource_exists::<ShowWorldInspector>))
        .add_systems(Update, toggle_world_inspector)
        // .add_plugins(ResourceInspectorPlugin::<PlayerMoveConfig>::default())
        .add_plugins(PanCamPlugin)
        .add_plugins(TilemapPlugin)
        // .add_plugins(terrain::TerrainPlugin)
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .add_plugins(main_menu::MainMenu)
        .add_plugins(space_shooter::Game)
        .add_plugins(factory_game::FactoryGamePlugin)
        .add_systems(OnEnter(GameState::MainMenu), setup_camera)
        .run();
}

#[derive(Resource)]
struct ShowWorldInspector;

fn toggle_world_inspector(
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    show: Option<Res<ShowWorldInspector>>,
) {
    if keys.clear_just_released(KeyCode::F12) {
        if show.is_some() {
            commands.remove_resource::<ShowWorldInspector>();
        } else {
            commands.insert_resource(ShowWorldInspector);
        }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        StateScoped(GameState::MainMenu),
        Camera2d,
        PrimaryEguiContext,
    ));
}
