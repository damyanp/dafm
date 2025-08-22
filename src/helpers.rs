use bevy::{ecs::query::QueryData, math::bounding::Aabb2d, prelude::*};
use bevy_ecs_tilemap::prelude::*;
use bevy_pancam::PanCam;

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct TilemapQuery {
    pub entity: Entity,
    pub map_size: &'static TilemapSize,
    pub grid_size: &'static TilemapGridSize,
    pub tile_size: &'static TilemapTileSize,
    pub map_type: &'static TilemapType,
    pub anchor: &'static TilemapAnchor,
}

impl TilemapQueryItem<'_> {
    pub fn anchor_offset(&self) -> Vec2 {
        self.anchor
            .as_offset(self.map_size, self.grid_size, self.tile_size, self.map_type)
    }

    pub fn center_in_world(&self, tile_pos: &TilePos) -> Vec2 {
        tile_pos.center_in_world(
            self.map_size,
            self.grid_size,
            self.tile_size,
            self.map_type,
            self.anchor,
        )
    }

    pub fn get_tile_pos_from_world_pos(&self, p: &Vec2) -> Option<TilePos> {
        TilePos::from_world_pos(
            p,
            self.map_size,
            self.grid_size,
            self.tile_size,
            self.map_type,
            self.anchor,
        )
    }
}

pub fn set_camera_limits_from_tilemaps(
    mut pan_cam: Single<&mut PanCam>,
    tilemaps: Query<(TilemapQuery, &Transform)>,
) {
    let combined_bounds = tilemaps
        .iter()
        .map(|(t, transform)| {
            // Calculate tilemap dimensions in world space
            let map_width = (t.map_size.x - 1) as f32 * t.tile_size.x;
            let map_height = (t.map_size.y - 1) as f32 * t.tile_size.y;

            // Use the existing anchor offset method
            let anchor_offset = t.anchor_offset();

            let half_tile_size = Vec2::from(t.tile_size) / 2.0;

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
                    let world_pos = transform.transform_point((corner + anchor_offset).extend(0.0));
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
        })
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
