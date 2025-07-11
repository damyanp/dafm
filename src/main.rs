use bevy::{math::bounding::Aabb2d, prelude::*, window::PresentMode};
use bevy_ecs_tilemap::prelude::*;

mod mapgen;
mod mapgen_viz;
use bevy_pancam::{PanCam, PanCamPlugin};
use mapgen_viz::MapGenPlugin;

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
        .add_plugins(MapGenPlugin)
        // .add_plugins(map::MapPlugin)
        .add_systems(Startup, startup)
        .add_systems(PostStartup, set_camera_limits)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn((Camera2d, PanCam::default()));
}

fn set_camera_limits(
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

mod map {
    use crate::mapgen;
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::*;

    pub struct MapPlugin;

    impl Plugin for MapPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, startup);
        }
    }

    fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
        let texture = asset_server.load("kentangpixel/SummerFloor.png");

        let map_size = TilemapSize { x: 256, y: 256 };

        let map = {
            info!("Generating map....");
            let mut generator = mapgen::Generator::new(&map_size);

            loop {
                while !generator.step() {}

                let result = generator.get();
                if result
                    .iter()
                    .all(|t| t.state != mapgen::TileState::Collapsed(0))
                {
                    info!("...done");
                    break result;
                }

                info!("...trying again");
                generator.reset();
            }
        };

        let tilemap_entity = commands.spawn_empty().id();

        let mut tile_storage = TileStorage::empty(map_size);

        let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();

        for x in 0..map_size.x {
            for y in 0..map_size.y {
                let tile_pos = TilePos { x, y };

                let tile = &map[tile_pos.to_index(&map_size)];
                let tile_index = match tile.state {
                    mapgen::TileState::Collapsed(i) => i,
                    mapgen::TileState::Options(_) => 0,
                };

                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(tile_index),
                        ..default()
                    })
                    .id();
                tile_storage.set(&tile_pos, tile_entity);
            }
        }

        commands.entity(tilemap_entity).insert(TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(texture),
            tile_size,
            anchor: TilemapAnchor::Center,
            ..default()
        });
    }
}
