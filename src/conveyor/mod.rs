use std::ops::DerefMut;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    prelude::*,
};
use tiled::Tile;

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
                    on_toggle_show_conveyors.run_if(input_just_pressed(KeyCode::Tab)),
                    on_test_data.run_if(input_just_pressed(KeyCode::KeyT)),
                )
                    .chain()
                    .run_if(in_state(GameState::Conveyor)),
            )
            .add_observer(update_conveyor_tiles);
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
    hovered_tile: Single<(&TilePos, &HoveredTile)>,
    mut base: Single<(Entity, &mut TileStorage), With<BaseLayer>>,
) {
    let (tilemap, storage) = base.deref_mut();

    let (tile_pos, hovered_tile) = *hovered_tile;
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
                        tilemap_id: TilemapId(*tilemap),
                        position: *tile_pos,
                        ..default()
                    },
                ))
                .id(),
        );
    }

    commands.trigger(ConveyorChanged(*tile_pos));
}

fn on_test_data(
    mut commands: Commands,
    mut base: Single<(Entity, &mut TileStorage), With<BaseLayer>>,
) {
    let (tilemap, storage) = base.deref_mut();

    let mut tile_pos = TilePos { x: 32, y: 58 };
    let dirs = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ];

    let mut spawn = |pos, direction| {
        storage.set(
            &pos,
            commands
                .spawn((
                    StateScoped(GameState::Conveyor),
                    Name::new("Test Data Tile"),
                    Conveyor(direction),
                    TileBundle {
                        tilemap_id: TilemapId(*tilemap),
                        position: pos.clone(),
                        ..default()
                    },
                ))
                .id(),
        );
        commands.trigger(ConveyorChanged(pos));
    };

    for a in &dirs {
        for b in &dirs {
            spawn(tile_pos, *a);

            match a {
                Direction::North => spawn(
                    TilePos {
                        x: tile_pos.x,
                        y: tile_pos.y + 1,
                    },
                    *b,
                ),
                Direction::South => spawn(
                    TilePos {
                        x: tile_pos.x,
                        y: tile_pos.y - 1,
                    },
                    *b,
                ),
                Direction::East => spawn(
                    TilePos {
                        x: tile_pos.x + 1,
                        y: tile_pos.y,
                    },
                    *b,
                ),
                Direction::West => spawn(
                    TilePos {
                        x: tile_pos.x - 1,
                        y: tile_pos.y,
                    },
                    *b,
                ),
            }

            tile_pos.x = tile_pos.x + 4;

            if tile_pos.x > 68 {
                tile_pos.x = 32;
                tile_pos.y -= 4;
            }
        }
    }
}

fn on_space(mut hovered_tile: Single<&mut HoveredTile>) {
    hovered_tile.set_to_next_option();
}

fn update_hovered_tile(
    mut q: Single<(Entity, &HoveredTile, &mut TileTextureIndex, &mut TileFlip)>,
) {
    if let HoveredTile(Some(hovered_direction)) = q.1 {
        let (_, _, texture_index, flip) = q.deref_mut();
        // let (storage, map_size) = *base;

        // let neighbors = Neighbors::get_square_neighboring_positions(tile_pos, map_size, false)
        //     .entities(storage);

        // let incoming_neighbor = neighbors.iter_with_direction().find(|(dir, entity)| {
        //     if let Ok(Conveyor(neighbor_dir)) = conveyors.get(**entity) {
        //         *neighbor_dir == opposite((*dir).into())
        //     } else {
        //         false
        //     }
        // });

        // let (from, to) = if let Some((incoming_direction, _)) = incoming_neighbor {
        //     (incoming_direction.into(), *hovered_direction)
        // } else {
        //     (opposite(*hovered_direction), *hovered_direction)
        // };
        (**texture_index, **flip) = get_hover_tile(*hovered_direction);
    } else {
        *q.2 = TileTextureIndex(20);
    }
}

#[derive(Component)]
struct DirectionArrow;

fn on_toggle_show_conveyors(
    mut commands: Commands,
    arrows: Query<Entity, With<DirectionArrow>>,
    interaction_layer: Single<Entity, With<InteractionLayer>>,
    conveyors: Query<(&Conveyor, &TilePos)>,
    mut enabled: Local<bool>,
) {
    *enabled = !*enabled;

    if *enabled {
        for (conveyor, tile_pos) in conveyors {
            let flip = match conveyor.0 {
                Direction::North => TileFlip {
                    y: true,
                    d: true,
                    ..default()
                },
                Direction::South => TileFlip {
                    d: true,
                    ..default()
                },
                Direction::East => TileFlip::default(),
                Direction::West => TileFlip {
                    x: true,
                    ..default()
                },
            };

            commands.spawn((
                StateScoped(GameState::Conveyor),
                Name::new("ConveyorDirection"),
                DirectionArrow,
                TileBundle {
                    texture_index: TileTextureIndex(22),
                    tilemap_id: TilemapId(*interaction_layer),
                    flip,
                    position: *tile_pos,
                    ..default()
                },
            ));
        }
    } else {
        arrows
            .iter()
            .for_each(|arrow| commands.entity(arrow).despawn());
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

#[derive(Event)]
struct ConveyorChanged(TilePos);

fn update_conveyor_tiles(
    trigger: Trigger<ConveyorChanged>,
    mut conveyor_tiles: Query<(&TilePos, &Conveyor, &mut TileTextureIndex, &mut TileFlip)>,
    conveyors: Query<&Conveyor>,
    mut base: Single<(&mut TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.deref_mut();

    update_conveyor_tile(
        &trigger.0,
        &tile_storage,
        map_size,
        &mut conveyor_tiles,
        &conveyors,
    );

    for neighbor in Neighbors::get_square_neighboring_positions(&trigger.0, map_size, false).iter()
    {
        update_conveyor_tile(
            neighbor,
            &tile_storage,
            map_size,
            &mut conveyor_tiles,
            &conveyors,
        );
    }
}

fn update_conveyor_tile(
    tile_pos: &TilePos,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    mut conveyor_tiles: &mut Query<(&TilePos, &Conveyor, &mut TileTextureIndex, &mut TileFlip)>,
    conveyors: &Query<&Conveyor>,
) {
    let this_conveyor = tile_storage
        .get(tile_pos)
        .and_then(|entity| conveyor_tiles.get_mut(entity).ok());

    if let Some((_, Conveyor(out_dir), mut texture_index, mut flip)) = this_conveyor {
        println!("Update {tile_pos:?} {out_dir:?}");

        // Find the neighbors that have conveyors on them
        let neighbor_conveyors =
            get_neighbor_conveyors(&tile_storage, tile_pos, &map_size, &conveyors);

        println!("  {neighbor_conveyors:?} - neighbors");

        // And just the conveyors pointing towards this one
        let neighbor_conveyors = Neighbors::from_directional_closure(|dir| {
            neighbor_conveyors.get(dir).and_then(|c| {
                if c.0 == opposite(dir.into()) {
                    Some(c.clone())
                } else {
                    None
                }
            })
        });

        println!("  {neighbor_conveyors:?} - pointing at us");

        // Rotate all of this so that east is always the "out" direction
        let neighbor_conveyors = make_east_relative(neighbor_conveyors, *out_dir);
        println!("  {neighbor_conveyors:?} - east relative");

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

        println!("{new_texture_index:?}, {y_flip:?}");

        *texture_index = new_texture_index;
        // let y_flip = false;
        // *texture_index = SOUTH_TO_EAST;

        *flip = match out_dir {
            Direction::North => TileFlip {
                d: true,
                ..default()
            },
            Direction::South => TileFlip {
                d: true,
                ..default()
            },
            Direction::East => TileFlip { ..default() },
            Direction::West => TileFlip {
                x: true,
                ..default()
            },
        };
    }
}

fn steps_from_east(direction: Direction) -> u32 {
    match direction {
        Direction::North => 1,
        Direction::South => 3,
        Direction::East => 0,
        Direction::West => 2,
    }
}

fn make_east_relative<T>(neighbors: Neighbors<T>, direction: Direction) -> Neighbors<T>
where
    T: Default,
{
    match direction {
        Direction::North => Neighbors {
            north: neighbors.west,
            east: neighbors.north,
            south: neighbors.east,
            west: neighbors.south,
            ..default()
        },
        Direction::East => neighbors,
        Direction::South => Neighbors {
            north: neighbors.east,
            east: neighbors.south,
            south: neighbors.west,
            west: neighbors.north,
            ..default()
        },
        Direction::West => Neighbors {
            north: neighbors.south,
            east: neighbors.west,
            south: neighbors.north,
            west: neighbors.east,
            ..default()
        },
    }
}

fn get_neighbor_conveyors(
    tile_storage: &TileStorage,
    tile_pos: &TilePos,
    map_size: &TilemapSize,
    conveyors: &Query<&Conveyor>,
) -> Neighbors<Conveyor> {
    let neighbor_positions = Neighbors::get_square_neighboring_positions(tile_pos, map_size, false);
    let neighbor_entities = neighbor_positions.entities(&tile_storage);

    neighbor_entities
        .and_then_ref(|n| conveyors.get(*n).ok())
        .map_ref(|c| (*c).clone())
}

const WEST_TO_EAST: TileTextureIndex = TileTextureIndex(11);
const SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(12);
const SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(13);
const NORTH_AND_SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(14);
const NORTH_AND_SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(15);

fn get_hover_tile(direction: Direction) -> (TileTextureIndex, TileFlip) {
    get_conveyor_tile(opposite(direction), direction)
}

fn get_conveyor_tile(from: Direction, to: Direction) -> (TileTextureIndex, TileFlip) {
    match (from, to) {
        // straights
        (Direction::West, Direction::East) | (Direction::East, Direction::East) => (
            WEST_TO_EAST,
            TileFlip {
                x: false,
                y: false,
                d: false,
            },
        ),
        (Direction::East, Direction::West) | (Direction::West, Direction::West) => (
            WEST_TO_EAST,
            TileFlip {
                x: true,
                y: false,
                d: false,
            },
        ),
        (Direction::North, Direction::South) | (Direction::South, Direction::South) => (
            WEST_TO_EAST,
            TileFlip {
                x: false,
                y: false,
                d: true,
            },
        ),
        (Direction::South, Direction::North) | (Direction::North, Direction::North) => (
            WEST_TO_EAST,
            TileFlip {
                x: false,
                y: true,
                d: true,
            },
        ),

        // corners
        (Direction::East, Direction::North) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                y: true,
                ..default()
            },
        ),
        (Direction::East, Direction::South) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                ..default()
            },
        ),
        (Direction::North, Direction::East) => (
            SOUTH_TO_EAST,
            TileFlip {
                y: true,
                ..default()
            },
        ),
        (Direction::North, Direction::West) => (
            SOUTH_TO_EAST,
            TileFlip {
                x: true,
                y: true,
                ..default()
            },
        ),
        (Direction::West, Direction::North) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                x: true,
                y: true,
                ..default()
            },
        ),
        (Direction::West, Direction::South) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                x: true,
                ..default()
            },
        ),
        (Direction::South, Direction::East) => (SOUTH_TO_EAST, TileFlip::default()),
        (Direction::South, Direction::West) => (
            SOUTH_TO_EAST,
            TileFlip {
                x: true,
                ..default()
            },
        ),
    }
}

#[derive(PartialEq, Reflect, Clone, Copy, Debug, Default)]
enum Direction {
    #[default]
    North,
    South,
    East,
    West,
}

#[derive(Component, Clone, Debug, Reflect, Default)]
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
