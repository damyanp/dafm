use std::ops::DerefMut;

use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::{Neighbors, SquareDirection};
use bevy_ecs_tilemap::prelude::*;

use super::{Conveyor, MapConfig, helpers::*, make_layer};
use crate::GameState;

pub struct Visuals;
impl Plugin for Visuals {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Conveyor), startup)
            .add_systems(Update, update_conveyor_tiles);
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
    new_conveyor_tiles: Query<(Entity, &TilePos), (With<Conveyor>, Without<TileTextureIndex>)>,
    conveyor_tiles: Query<(
        Entity,
        &Conveyor,
        Option<&TileTextureIndex>,
        Option<&TileFlip>,
    )>,
    conveyors: Query<&Conveyor>,
    mut base: Single<(Entity, &mut TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tilemap_entity, tile_storage, map_size) = base.deref_mut();

    for (new_conveyor_entity, new_conveyor_pos) in new_conveyor_tiles {
        commands
            .entity(new_conveyor_entity)
            .insert_if_new(TileBundle {
                tilemap_id: TilemapId(*tilemap_entity),
                ..default()
            });

        update_conveyor_tile(
            commands.reborrow(),
            new_conveyor_pos,
            tile_storage,
            map_size,
            &conveyor_tiles,
            &conveyors,
        );

        for neighbor in
            Neighbors::get_square_neighboring_positions(new_conveyor_pos, map_size, false).iter()
        {
            update_conveyor_tile(
                commands.reborrow(),
                neighbor,
                tile_storage,
                map_size,
                &conveyor_tiles,
                &conveyors,
            );
        }
    }
}

fn update_conveyor_tile(
    mut commands: Commands,
    tile_pos: &TilePos,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    conveyor_tiles: &Query<(
        Entity,
        &Conveyor,
        Option<&TileTextureIndex>,
        Option<&TileFlip>,
    )>,
    conveyors: &Query<&Conveyor>,
) {
    let this_conveyor = tile_storage
        .get(tile_pos)
        .and_then(|entity| conveyor_tiles.get(entity).ok());

    if let Some((entity, Conveyor(out_dir), texture_index, flip)) = this_conveyor {
        let out_dir: SquareDirection = (*out_dir).into();

        // Find the neighbors that have conveyors on them
        let neighbor_conveyors =
            get_neighbors_from_query(tile_storage, tile_pos, map_size, conveyors);

        // And just the conveyors pointing towards this one
        let neighbor_conveyors = Neighbors::from_directional_closure(|dir| {
            neighbor_conveyors.get(dir).and_then(|c| {
                if c.0 == opposite(dir).into() {
                    Some(c.clone())
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
}

const WEST_TO_EAST: TileTextureIndex = TileTextureIndex(11);
const SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(12);
const SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(13);
const NORTH_AND_SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(14);
const NORTH_AND_SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(15);
