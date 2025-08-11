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

#[derive(PartialEq, Eq, Reflect, Clone, Copy, Debug, Default)]
pub enum ConveyorDirection {
    #[default]
    North,
    South,
    East,
    West,
}

impl ConveyorDirection {
    #[allow(dead_code)]
    pub fn opposite(&self) -> Self {
        use ConveyorDirection::*;
        match self {
            East => West,
            North => South,
            West => East,
            South => North,
        }
    }
}

#[derive(PartialEq, Eq, Reflect, Clone, Copy, Debug, Default)]
pub struct ConveyorDirections(u8);

impl ConveyorDirections {
    pub fn new(direction: ConveyorDirection) -> Self {
        Self(direction.into())
    }

    pub fn all() -> Self {
        use ConveyorDirection::*;

        let n: u8 = North.into();
        let e: u8 = East.into();
        let s: u8 = South.into();
        let w: u8 = West.into();

        Self(n | e | s | w)
    }

    pub fn is_set(&self, direction: ConveyorDirection) -> bool {
        let direction: u8 = direction.into();
        (self.0 & direction) != 0u8
    }

    pub fn single(&self) -> ConveyorDirection {
        let mut iter = self.iter();
        let direction = iter.next().unwrap();
        if iter.next().is_some() {
            panic!("Expected exactly one direction");
        }
        direction
    }

    pub fn iter(&self) -> impl Iterator<Item = ConveyorDirection> {
        CONVEYOR_DIRECTIONS.into_iter().filter(|d| self.is_set(*d))
    }

    pub fn iter_from(
        &self,
        direction: ConveyorDirection,
    ) -> impl Iterator<Item = ConveyorDirection> {
        let direction = direction.index();
        (0..CONVEYOR_DIRECTIONS.len())
            .map(move |i| (direction + i) % CONVEYOR_DIRECTIONS.len())
            .map(|i| CONVEYOR_DIRECTIONS[i])
            .filter(|d| self.is_set(*d))
    }
}

impl From<ConveyorDirection> for u8 {
    fn from(value: ConveyorDirection) -> Self {
        match value {
            ConveyorDirection::North => 1,
            ConveyorDirection::East => 2,
            ConveyorDirection::South => 4,
            ConveyorDirection::West => 8,
        }
    }
}

pub const CONVEYOR_DIRECTIONS: [ConveyorDirection; 4] = [
    ConveyorDirection::North,
    ConveyorDirection::East,
    ConveyorDirection::South,
    ConveyorDirection::West,
];

impl ConveyorDirection {
    pub fn index(&self) -> usize {
        use ConveyorDirection::*;
        match self {
            North => 0,
            East => 1,
            South => 2,
            West => 3,
        }
    }

    pub fn next(&self) -> ConveyorDirection {
        use ConveyorDirection::*;
        match self {
            North => East,
            East => South,
            South => West,
            West => North,
        }
    }
}

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
