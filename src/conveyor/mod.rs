use std::ops::DerefMut;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::prelude::*;

use crate::GameState;

pub struct ConveyorPlugin;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapConfig::default())
            .add_systems(OnEnter(GameState::Conveyor), startup)
            .add_systems(Update, track_mouse)
            .add_systems(
                Update,
                (
                    on_click.run_if(input_just_pressed(MouseButton::Left)),
                    on_space.run_if(input_just_pressed(KeyCode::Space)),
                )
                    .run_if(in_state(GameState::Conveyor)),
            );
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<MapConfig>) {
    let texture = asset_server.load("sprites.png");
    let interaction_layer = commands
        .spawn(make_interaction_layer(&config, texture.to_owned()))
        .id();

    commands.spawn((
        StateScoped(GameState::Conveyor),
        Name::new("HoveredTile"),
        HoveredTile(None),
        TileBundle {
            texture_index: TileTextureIndex(20),
            tilemap_id: TilemapId(interaction_layer),
            ..default()
        },
    ));

    commands.spawn(make_base_layer(&config, texture.to_owned()));
}

#[allow(clippy::type_complexity)]
fn track_mouse(
    mut cursor_moved: EventReader<CursorMoved>,
    camera_query: Single<(&GlobalTransform, &Camera)>,
    interaction_layer: Single<
        (
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<InteractionLayer>,
    >,
    mut hovered_tile: Single<&mut TilePos, With<HoveredTile>>,
) {
    if let Some(e) = cursor_moved.read().last() {
        let (global_transform, camera) = *camera_query;
        if let Ok(p) = camera.viewport_to_world_2d(global_transform, e.position) {
            let (size, grid_size, tile_size, map_type, anchor) = *interaction_layer;

            if let Some(tile_pos) =
                TilePos::from_world_pos(&p, size, grid_size, tile_size, map_type, anchor)
            {
                **hovered_tile = tile_pos;
            }
        }
    }
}

fn on_click(
    mut commands: Commands,
    hovered_tile: Single<(&TilePos, &TileTextureIndex, &TileFlip, &HoveredTile), With<HoveredTile>>,
    mut base: Single<(Entity, &mut TileStorage), With<BaseLayer>>,
) {
    let (tilemap, storage) = base.deref_mut();

    let (tile_pos, tile_texture_index, tile_flip, hovered_tile) = *hovered_tile;
    if hovered_tile.0.is_none() {
        if let Some(e) = storage.get(tile_pos) {
            storage.remove(tile_pos);
            commands.entity(e).despawn();
        }
    } else {
        storage.set(
            tile_pos,
            commands
                .spawn((
                    StateScoped(GameState::Conveyor),
                    Name::new("Placed Tile"),
                    TileBundle {
                        texture_index: *tile_texture_index,
                        flip: *tile_flip,
                        tilemap_id: TilemapId(*tilemap),
                        position: *tile_pos,
                        ..default()
                    },
                ))
                .id(),
        );
    }
}

fn on_space(mut q: Single<(&mut TileTextureIndex, &mut TileFlip, &mut HoveredTile)>) {
    let (tti, tf, ht) = q.deref_mut();

    ht.set_to_next_option();

    if let HoveredTile(Some(flip)) = **ht {
        **tti = TileTextureIndex(11);
        **tf = flip;
    } else {
        **tti = TileTextureIndex(20);
    }
}

#[derive(Component)]
struct HoveredTile(Option<TileFlip>);

impl HoveredTile {
    fn set_to_next_option(&mut self) {
        self.0 = match self.0 {
            None => Some(TileFlip {
                d: false,
                x: false,
                y: false,
            }),
            Some(TileFlip {
                d: false,
                x: false,
                y: false,
            }) => Some(TileFlip {
                d: true,
                x: false,
                y: false,
            }),
            Some(TileFlip {
                d: true,
                x: false,
                y: false,
            }) => Some(TileFlip {
                d: false,
                x: true,
                y: false,
            }),
            Some(TileFlip {
                d: false,
                x: true,
                y: false,
            }) => Some(TileFlip {
                d: true,
                x: false,
                y: true,
            }),
            _ => None,
        };
    }
}

#[derive(Component)]
struct InteractionLayer;

fn make_interaction_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (
        InteractionLayer,
        make_layer(config, texture, "InteractionLayer"),
    )
}

#[derive(Component)]
struct BaseLayer;

fn make_base_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (BaseLayer, make_layer(config, texture, "BaseLayer"))
}

fn make_layer(config: &MapConfig, texture: Handle<Image>, name: &'static str) -> impl Bundle {
    (
        StateScoped(GameState::Conveyor),
        Name::new(name),
        TilemapBundle {
            size: config.size,
            tile_size: config.tile_size,
            grid_size: config.grid_size,
            map_type: config.map_type,
            anchor: TilemapAnchor::Center,
            texture: TilemapTexture::Single(texture),
            storage: TileStorage::empty(config.size),
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
