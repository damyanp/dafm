use avian2d::prelude::*;
use bevy::{prelude::*, window::PresentMode};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_enhanced_input::EnhancedInputPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rand::plugin::EntropyPlugin;
use bevy_rand::prelude::*;

mod conveyor;
mod main_menu;
mod space_shooter;
mod terrain;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum GameState {
    #[default]
    MainMenu,
    SpaceShooter,
    Conveyor,
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
        // .add_plugins(PanCamPlugin::default())
        .add_plugins(TilemapPlugin)
        // .add_plugins(terrain::TerrainPlugin)
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .add_plugins(main_menu::MainMenu)
        .add_plugins(space_shooter::Game)
        .add_plugins(conveyor::ConveyorPlugin)
        .add_systems(Startup, startup)
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

fn startup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
