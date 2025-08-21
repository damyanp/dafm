// Experiments with terrain generation / wave form collapse etc.
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

mod map;
mod mapgen;
mod mapgen_viz;

use mapgen_viz::MapGenPlugin;

use crate::helpers::set_camera_limits_from_tilemaps;

pub struct TerrainPlugin;

pub fn terrain_plugin(app: &mut App) {
    assert!(app.is_plugin_added::<TilemapPlugin>());
    app.add_plugins(MapGenPlugin)
        // .add_plugins(map::MapPlugin)
        .add_systems(PostStartup, set_camera_limits_from_tilemaps);
}
