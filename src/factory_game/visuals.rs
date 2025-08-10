use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::{Neighbors, SquareDirection};
use bevy_ecs_tilemap::prelude::*;

use crate::factory_game::conveyor::Conveyor;
use crate::{
    GameState,
    factory_game::{ConveyorSystems, MapConfig, helpers::*, make_layer},
};

pub struct VisualsPlugin;
impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::FactoryGame), startup)
            .add_systems(
                Update,
                update_conveyor_tiles.in_set(ConveyorSystems::TileUpdater),
            )
            .add_systems(PostUpdate, cleanup_tiles);
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<MapConfig>) {
    let texture = asset_server.load("sprites.png");
    commands.spawn(make_base_layer(&config, texture.to_owned()));
}

#[derive(Component)]
pub struct BaseLayer;

fn make_base_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (BaseLayer, make_layer(config, texture, 0.0, "BaseLayer"))
}

#[allow(clippy::type_complexity)]
fn update_conveyor_tiles(
    mut commands: Commands,
    new_conveyors: Query<&TilePos, (With<Conveyor>, Without<TileTextureIndex>)>,
    mut removed_conveyors: RemovedComponents<Conveyor>,
    conveyors: Query<(&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>)>,
    tiles: Query<&TilePos>,
    base: Single<(Entity, &TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tilemap_entity, tile_storage, map_size) = base.into_inner();

    let mut to_check = HashSet::new();

    new_conveyors.iter().for_each(|pos| {
        to_check.insert(*pos);
    });
    removed_conveyors.read().for_each(|entity| {
        to_check.insert(*tiles.get(entity).unwrap());
    });

    let sources: Vec<_> = to_check.iter().cloned().collect();
    for pos in sources {
        for neighbor_pos in
            Neighbors::get_square_neighboring_positions(&pos, map_size, false).iter()
        {
            to_check.insert(*neighbor_pos);
        }
    }

    for pos in to_check {
        if let Some(entity) = tile_storage.get(&pos) {
            if let Ok(conveyor) = conveyors.get(entity) {
                commands.entity(entity).insert_if_new(TileBundle {
                    tilemap_id: TilemapId(tilemap_entity),
                    ..default()
                });

                update_conveyor_tile(
                    commands.reborrow(),
                    entity,
                    conveyor,
                    &pos,
                    tile_storage,
                    map_size,
                    &conveyors,
                );
            }
        }
    }
}

fn update_conveyor_tile(
    mut commands: Commands,
    entity: Entity,
    conveyor: (&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>),
    tile_pos: &TilePos,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    conveyors: &Query<(&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>)>,
) {
    let (Conveyor(out_dir), texture_index, flip) = conveyor;

    let out_dir: SquareDirection = (*out_dir).into();

    // Find the neighbors that have conveyors on them
    let neighbor_conveyors = get_neighbors_from_query(tile_storage, tile_pos, map_size, conveyors);

    // And just the conveyors pointing towards this one
    let neighbor_conveyors = Neighbors::from_directional_closure(|dir| {
        neighbor_conveyors.get(dir).and_then(|c| {
            if c.0.0 == opposite(dir).into() {
                Some(*c)
            } else {
                None
            }
        })
    });

    // Rotate all of this so that east is always the "out" direction
    let neighbor_conveyors = make_east_relative(neighbor_conveyors, out_dir);

    let (new_texture_index, y_flip) = match neighbor_conveyors {
        Neighbors {
            north: None,
            east: None,
            south: None,
            west: Some(_),
            ..
        } => (WEST_TO_EAST, false),
        Neighbors {
            north: None,
            east: None,
            south: Some(_),
            west: Some(_),
            ..
        } => (SOUTH_AND_WEST_TO_EAST, false),
        Neighbors {
            north: Some(_),
            east: None,
            south: None,
            west: Some(_),
            ..
        } => (SOUTH_AND_WEST_TO_EAST, true),
        Neighbors {
            north: None,
            east: None,
            south: Some(_),
            west: None,
            ..
        } => (SOUTH_TO_EAST, false),
        Neighbors {
            north: Some(_),
            east: None,
            south: None,
            west: None,
            ..
        } => (SOUTH_TO_EAST, true),
        Neighbors {
            north: Some(_),
            east: None,
            south: Some(_),
            west: None,
            ..
        } => (NORTH_AND_SOUTH_TO_EAST, false),
        Neighbors {
            north: Some(_),
            east: None,
            south: Some(_),
            west: Some(_),
            ..
        } => (NORTH_AND_SOUTH_AND_WEST_TO_EAST, false),
        _ => (WEST_TO_EAST, false),
    };

    // y_flip indicates if we should flip y for the "east is always out"
    // orientation.  Now we need to rotate the tile so that the out
    // direction is correct.  For North/South this means that y_flip
    // actually becomes an x_flip.
    let new_flip = match out_dir {
        SquareDirection::North => TileFlip {
            x: y_flip,
            y: true,
            d: true,
        },
        SquareDirection::South => TileFlip {
            x: !y_flip,
            y: false,
            d: true,
        },
        SquareDirection::East => TileFlip {
            x: false,
            y: y_flip,
            d: false,
        },
        SquareDirection::West => TileFlip {
            x: true,
            y: !y_flip,
            d: false,
        },
        _ => panic!(),
    };

    if Some(&new_texture_index) != texture_index || Some(&new_flip) != flip {
        commands
            .entity(entity)
            .insert((new_texture_index, new_flip));
    }
}

fn cleanup_tiles(
    mut commands: Commands,
    mut storage: Single<&mut TileStorage, With<BaseLayer>>,
    dead_tiles: Query<&TilePos, (With<BaseLayer>, Without<TileTextureIndex>)>,
) {
    for dead_tile in dead_tiles {
        if let Some(entity) = storage.remove(dead_tile) {
            commands.entity(entity).despawn();
        }
    }
}

const WEST_TO_EAST: TileTextureIndex = TileTextureIndex(11);
const SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(12);
const SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(13);
const NORTH_AND_SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(14);
const NORTH_AND_SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(15);
