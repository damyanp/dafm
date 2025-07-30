use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use tiled::Tile;

use crate::GameState;

pub struct ConveyorPlugin;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapConfig::default())
            .add_systems(OnEnter(GameState::Conveyor), startup)
            .add_systems(Update, track_mouse);
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<MapConfig>) {
    commands.spawn(make_interaction_layer(
        &config,
        asset_server.load("sprites.png"),
    ));
}

fn track_mouse(
    mut commands: Commands,
    mut cursor_moved: EventReader<CursorMoved>,
    camera_query: Single<(&GlobalTransform, &Camera)>,
    mut interaction_layer: Single<
        (
            Entity,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<InteractionLayer>,
    >,
    mut current_hovered_tile: Option<Single<&mut TilePos, With<HoveredTile>>>,
) {
    let (global_transform, camera) = *camera_query;
    if let Some(e) = cursor_moved.read().last() {
        if let Ok(p) = camera.viewport_to_world_2d(global_transform, e.position) {
            let (entity, size, grid_size, tile_size, map_type, anchor) = &mut (*interaction_layer);

            if let Some(tile_pos) =
                TilePos::from_world_pos(&p, size, grid_size, tile_size, map_type, anchor)
            {
                if let Some(old_pos) = &mut current_hovered_tile {
                    ***old_pos = tile_pos;
                } else {
                    commands.spawn((
                        HoveredTile,
                        TileBundle {
                            position: tile_pos,
                            texture_index: TileTextureIndex(20),
                            tilemap_id: TilemapId(*entity),
                            ..default()
                        },
                    ));
                }
            }
        }
    }
}

#[derive(Component)]
struct HoveredTile;

#[derive(Component)]
struct InteractionLayer;

fn make_interaction_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (
        StateScoped(GameState::Conveyor),
        Name::new("InteractionLayer"),
        InteractionLayer,
        TilemapBundle {
            size: config.size,
            tile_size: config.tile_size,
            grid_size: config.grid_size,
            map_type: config.map_type,
            anchor: TilemapAnchor::Center,
            texture: TilemapTexture::Single(texture),
            ..default()
        },
    )
}

#[derive(Resource)]
struct MapConfig {
    size: TilemapSize,
    tile_size: TilemapTileSize,
    grid_size: TilemapGridSize,
    map_type: TilemapType,
}

impl Default for MapConfig {
    fn default() -> Self {
        let map_size = TilemapSize { x: 5, y: 5 };
        let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
        let grid_size = tile_size.into();

        Self {
            size: map_size,
            tile_size,
            grid_size,
            map_type: Default::default(),
        }
    }
}
