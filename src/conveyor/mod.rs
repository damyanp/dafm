use std::ops::DerefMut;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    prelude::*,
};

use crate::GameState;

pub struct ConveyorPlugin;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Conveyor>()
            .register_type::<HoveredTile>()
            .insert_resource(MapConfig::default())
            .add_systems(OnEnter(GameState::Conveyor), startup)
            .add_systems(Update, track_mouse)
            .add_systems(
                Update,
                (
                    on_click.run_if(input_just_pressed(MouseButton::Left)),
                    on_space.run_if(input_just_pressed(KeyCode::Space)),
                    update_hovered_tile,
                )
                    .chain()
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

#[allow(clippy::type_complexity)]
fn on_click(
    mut commands: Commands,
    hovered_tile: Single<
        (
            &TilePos,
            &TileTextureIndex,
            &TileFlip,
            &HoveredTile,
            Option<&Conveyor>,
        ),
        With<HoveredTile>,
    >,
    mut base: Single<(Entity, &mut TileStorage), With<BaseLayer>>,
) {
    let (tilemap, storage) = base.deref_mut();

    let (tile_pos, tile_texture_index, tile_flip, hovered_tile, conveyor) = *hovered_tile;
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
                    Conveyor(hovered_tile.0.unwrap()),
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

fn on_space(mut hovered_tile: Single<&mut HoveredTile>) {
    hovered_tile.set_to_next_option();
}

fn update_hovered_tile(
    mut commands: Commands,
    mut q: Single<(
        Entity,
        &HoveredTile,
        &TilePos,
        &mut TileTextureIndex,
        &mut TileFlip,
    )>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    conveyors: Query<&Conveyor>,
) {
    if let HoveredTile(Some(hovered_direction)) = q.1 {
        let (_, _, tile_pos, texture_index, flip) = q.deref_mut();
        let (storage, map_size) = *base;

        let neighbors = Neighbors::get_square_neighboring_positions(tile_pos, map_size, false)
            .entities(storage);

        let incoming_neighbor = neighbors.iter_with_direction().find(|(dir, entity)| {
            if let Ok(Conveyor(neighbor_dir)) = conveyors.get(**entity) {
                *neighbor_dir == opposite((*dir).into())
            } else {
                false
            }
        });

        let (from, to) = if let Some((incoming_direction, _)) = incoming_neighbor {
            (incoming_direction.into(), *hovered_direction)
        } else {
            (opposite(*hovered_direction), *hovered_direction)
        };
        (**texture_index, **flip) = get_conveyor_tile(from, to);
    } else {
        *q.3 = TileTextureIndex(20);
    }
}

impl From<SquareDirection> for Direction {
    fn from(value: SquareDirection) -> Self {
        match value {
            SquareDirection::North => Direction::North,
            SquareDirection::East => Direction::East,
            SquareDirection::South => Direction::South,
            SquareDirection::West => Direction::West,
            _ => panic!(),
        }
    }
}

fn opposite(d: Direction) -> Direction {
    match d {
        Direction::East => Direction::West,
        Direction::North => Direction::South,
        Direction::West => Direction::East,
        Direction::South => Direction::North,
    }
}

fn get_conveyor_tile(from: Direction, to: Direction) -> (TileTextureIndex, TileFlip) {
    const STRAIGHT: TileTextureIndex = TileTextureIndex(11);
    const CORNER: TileTextureIndex = TileTextureIndex(13);
    match (from, to) {
        // straights
        (Direction::West, Direction::East) | (Direction::East, Direction::East) => (
            STRAIGHT,
            TileFlip {
                x: false,
                y: false,
                d: false,
            },
        ),
        (Direction::East, Direction::West) | (Direction::West, Direction::West) => (
            STRAIGHT,
            TileFlip {
                x: true,
                y: false,
                d: false,
            },
        ),
        (Direction::North, Direction::South) | (Direction::South, Direction::South) => (
            STRAIGHT,
            TileFlip {
                x: false,
                y: false,
                d: true,
            },
        ),
        (Direction::South, Direction::North) | (Direction::North, Direction::North) => (
            STRAIGHT,
            TileFlip {
                x: false,
                y: true,
                d: true,
            },
        ),

        // corners
        (Direction::East, Direction::North) => (
            CORNER,
            TileFlip {
                x: true,
                y: true,
                ..default()
            },
        ),
        (Direction::East, Direction::South) => (
            CORNER,
            TileFlip {
                x: true,
                ..default()
            },
        ),
        (Direction::North, Direction::East) => (
            CORNER,
            TileFlip {
                d: true,
                ..default()
            },
        ),
        (Direction::North, Direction::West) => (
            CORNER,
            TileFlip {
                d: true,
                x: true,
                ..default()
            },
        ),
        (Direction::West, Direction::North) => (
            CORNER,
            TileFlip {
                y: true,
                ..default()
            },
        ),
        (Direction::West, Direction::South) => (CORNER, TileFlip::default()),
        (Direction::South, Direction::East) => (
            CORNER,
            TileFlip {
                d: true,
                y: true,
                ..default()
            },
        ),
        (Direction::South, Direction::West) => (
            CORNER,
            TileFlip {
                d: true,
                x: true,
                y: true,
            },
        ),
    }
}

#[derive(PartialEq, Reflect, Clone, Copy, Debug)]
enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Component, Clone, Debug, Reflect)]
struct Conveyor(Direction);

#[derive(Component, Reflect)]
struct HoveredTile(Option<Direction>);

impl HoveredTile {
    fn set_to_next_option(&mut self) {
        self.0 = match self.0 {
            None => Some(Direction::East),
            Some(Direction::East) => Some(Direction::South),
            Some(Direction::South) => Some(Direction::West),
            Some(Direction::West) => Some(Direction::North),
            Some(Direction::North) => None,
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
