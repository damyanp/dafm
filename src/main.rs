use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

mod mapgen;
mod mapgen_viz;
use mapgen_viz::MapGenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(TilemapPlugin)
        .add_plugins(MapGenPlugin)
        .add_systems(Startup, startup)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

