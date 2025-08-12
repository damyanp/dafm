// Experiments with terrain generation / wave form collapse etc.
use bevy::{math::bounding::Aabb2d, prelude::*};
use bevy_ecs_tilemap::prelude::*;

mod map;
mod mapgen;
mod mapgen_viz;

use bevy_pancam::PanCam;
use mapgen_viz::MapGenPlugin;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<TilemapPlugin>());
        app.add_plugins(MapGenPlugin)
            // .add_plugins(map::MapPlugin)
            .add_systems(PostStartup, set_camera_limits);
    }
}

pub fn set_camera_limits(
    mut pan_cam: Single<&mut PanCam>,
    tilemaps: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
        &Transform,
    )>,
) {
    let combined_bounds = tilemaps
        .iter()
        .map(
            |(map_size, grid_size, tile_size, map_type, anchor, transform)| {
                // Calculate tilemap dimensions in world space
                let map_width = (map_size.x - 1) as f32 * tile_size.x;
                let map_height = (map_size.y - 1) as f32 * tile_size.y;

                // Use the existing anchor offset method
                let anchor_offset = anchor.as_offset(map_size, grid_size, tile_size, map_type);

                let half_tile_size = Vec2::from(tile_size) / 2.0;

                // Calculate the four corners of the tilemap
                let corners = [
                    Vec2::new(-half_tile_size.x, -half_tile_size.y),
                    Vec2::new(map_width + half_tile_size.x, -half_tile_size.y),
                    Vec2::new(-half_tile_size.x, map_height + half_tile_size.y),
                    Vec2::new(map_width + half_tile_size.x, map_height + half_tile_size.y),
                ];

                // Transform corners and find the bounding box
                let transformed_corners: Vec<Vec2> = corners
                    .iter()
                    .map(|&corner| {
                        let world_pos =
                            transform.transform_point((corner + anchor_offset).extend(0.0));
                        Vec2::new(world_pos.x, world_pos.y)
                    })
                    .collect();

                let min_corner = transformed_corners
                    .iter()
                    .fold(Vec2::INFINITY, |acc, &corner| acc.min(corner));
                let max_corner = transformed_corners
                    .iter()
                    .fold(Vec2::NEG_INFINITY, |acc, &corner| acc.max(corner));

                Aabb2d::new(
                    (min_corner + max_corner) / 2.0, // center
                    (max_corner - min_corner) / 2.0, // half_size
                )
            },
        )
        .reduce(|a, b| {
            let min_point = a.min.min(b.min);
            let max_point = a.max.max(b.max);
            Aabb2d::new((min_point + max_point) / 2.0, (max_point - min_point) / 2.0)
        })
        .unwrap_or_else(|| Aabb2d::new(Vec2::ZERO, Vec2::ZERO));

    // Set the camera limits
    pan_cam.min_x = combined_bounds.min.x;
    pan_cam.min_y = combined_bounds.min.y;
    pan_cam.max_x = combined_bounds.max.x;
    pan_cam.max_y = combined_bounds.max.y;
}
