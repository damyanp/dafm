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
            &mut TileStorage,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<InteractionLayer>,
    >,
    current_hovered_tile: Option<Single<(Entity, &HoveredTile, &mut TilePos)>>,
) {
    let (global_transform, camera) = *camera_query;
    if let Some(e) = cursor_moved.read().last() {
        if let Ok(p) = camera.viewport_to_world_2d(global_transform, e.position) {
            let (entity, storage, size, grid_size, tile_size, map_type, anchor) =
                &mut (*interaction_layer);

            if let Some(tile_pos) =
                TilePos::from_world_pos(&p, size, grid_size, tile_size, map_type, anchor)
            {
                if let Some(current) = current_hovered_tile {
                    commands.entity(current.0).despawn();
                }

                let tile_entity = commands.spawn((
                    HoveredTile,
                    TileBundle {
                        position: tile_pos,
                        texture_index: TileTextureIndex(20),
                        tilemap_id: TilemapId(*entity),
                        ..default()
                    },
                )).id();
                storage.set(&tile_pos, tile_entity);
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
            storage: TileStorage::empty(config.size),
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
        let map_size = TilemapSize { x: 100, y: 100 };
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
