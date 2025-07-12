use bevy::{prelude::*, window::PresentMode};
use bevy_ecs_tilemap::prelude::*;

mod terrain;

use bevy_pancam::{PanCam, PanCamPlugin};

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
        .add_plugins(PanCamPlugin::default())
        .add_plugins(TilemapPlugin)
        .add_plugins(terrain::TerrainPlugin)
        .add_systems(Startup, startup)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn((Camera2d, PanCam::default()));
}
