use bevy::ecs::query::{QueryData, QueryFilter, ROQueryItem};
use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::{Neighbors, SquareDirection};
use bevy_ecs_tilemap::prelude::*;

pub fn opposite(d: SquareDirection) -> SquareDirection {
    use SquareDirection::*;
    match d {
        East => West,
        North => South,
        West => East,
        South => North,
        _ => panic!(),
    }
}

#[derive(PartialEq, Reflect, Clone, Copy, Debug, Default)]
pub enum ConveyorDirection {
    #[default]
    North,
    South,
    East,
    West,
}

pub const CONVEYOR_DIRECTIONS: [ConveyorDirection; 4] = [
    ConveyorDirection::North,
    ConveyorDirection::East,
    ConveyorDirection::South,
    ConveyorDirection::West,
];

impl From<ConveyorDirection> for SquareDirection {
    fn from(value: ConveyorDirection) -> Self {
        match value {
            ConveyorDirection::North => SquareDirection::North,
            ConveyorDirection::South => SquareDirection::South,
            ConveyorDirection::East => SquareDirection::East,
            ConveyorDirection::West => SquareDirection::West,
        }
    }
}

impl From<SquareDirection> for ConveyorDirection {
    fn from(value: SquareDirection) -> Self {
        match value {
            SquareDirection::East => ConveyorDirection::East,
            SquareDirection::North => ConveyorDirection::North,
            SquareDirection::West => ConveyorDirection::West,
            SquareDirection::South => ConveyorDirection::South,
            _ => panic!(),
        }
    }
}

pub fn make_east_relative<T>(neighbors: Neighbors<T>, direction: SquareDirection) -> Neighbors<T> {
    match direction {
        SquareDirection::North => Neighbors {
            north: neighbors.west,
            east: neighbors.north,
            south: neighbors.east,
            west: neighbors.south,
            north_east: None,
            north_west: None,
            south_west: None,
            south_east: None,
        },
        SquareDirection::East => neighbors,
        SquareDirection::South => Neighbors {
            north: neighbors.east,
            east: neighbors.south,
            south: neighbors.west,
            west: neighbors.north,
            north_east: None,
            north_west: None,
            south_west: None,
            south_east: None,
        },
        SquareDirection::West => Neighbors {
            north: neighbors.south,
            east: neighbors.west,
            south: neighbors.north,
            west: neighbors.east,
            north_east: None,
            north_west: None,
            south_west: None,
            south_east: None,
        },
        _ => panic!(),
    }
}

pub fn get_neighbors_from_query<'a, D: QueryData, F: QueryFilter>(
    tile_storage: &TileStorage,
    tile_pos: &TilePos,
    map_size: &TilemapSize,
    query: &'a Query<D, F>,
) -> Neighbors<ROQueryItem<'a, D>> {
    let neighbor_positions = Neighbors::get_square_neighboring_positions(tile_pos, map_size, false);
    let neighbor_entities = neighbor_positions.entities(tile_storage);

    neighbor_entities.and_then_ref(|n| query.get(*n).ok())
}
