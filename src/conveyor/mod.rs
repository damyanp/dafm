use crate::GameState;
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::{
    helpers::square_grid::{
        SquarePos,
        neighbors::{CARDINAL_SQUARE_DIRECTIONS, Neighbors, SquareDirection},
    },
    prelude::*,
};
use std::ops::DerefMut;

pub struct ConveyorPlugin;

mod helpers;
use helpers::*;

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
    mut base: Single<(Entity, &mut TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tilemap, storage, map_size) = base.deref_mut();

    let mut pos = SquarePos { x: 32, y: 58 };

    let mut spawn = |pos: SquarePos, direction| {
        let pos = pos.as_tile_pos(map_size).unwrap();
        storage.set(
            &pos,
            commands
                .spawn((
                    StateScoped(GameState::Conveyor),
                    Name::new("Test Data Tile"),
                    Conveyor(direction),
                    TileBundle {
                        tilemap_id: TilemapId(*tilemap),
                        position: pos,
                        ..default()
                    },
                ))
                .id(),
        );
        commands.trigger(ConveyorChanged(pos));
    };

    for a in CARDINAL_SQUARE_DIRECTIONS {
        for b in CARDINAL_SQUARE_DIRECTIONS {
            spawn(pos, a.into());
            spawn(pos + a.into(), b.into());

            pos.x += 4;

            if pos.x > 68 {
                pos.x = 32;
                pos.y -= 4;
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
        (**texture_index, **flip) = get_hover_tile((*hovered_direction).into());
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
                ConveyorDirection::North => TileFlip {
                    y: true,
                    d: true,
                    ..default()
                },
                ConveyorDirection::South => TileFlip {
                    d: true,
                    ..default()
                },
                ConveyorDirection::East => TileFlip::default(),
                ConveyorDirection::West => TileFlip {
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
        tile_storage,
        map_size,
        &mut conveyor_tiles,
        &conveyors,
    );

    for neighbor in Neighbors::get_square_neighboring_positions(&trigger.0, map_size, false).iter()
    {
        update_conveyor_tile(
            neighbor,
            tile_storage,
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
    conveyor_tiles: &mut Query<(&TilePos, &Conveyor, &mut TileTextureIndex, &mut TileFlip)>,
    conveyors: &Query<&Conveyor>,
) {
    let this_conveyor = tile_storage
        .get(tile_pos)
        .and_then(|entity| conveyor_tiles.get_mut(entity).ok());

    if let Some((_, Conveyor(out_dir), mut texture_index, mut flip)) = this_conveyor {
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

        *texture_index = new_texture_index;

        // y_flip indicates if we should flip y for the "east is always out"
        // orientation.  Now we need to rotate the tile so that the out
        // direction is correct.  For North/South this means that y_flip
        // actually becomes an x_flip.
        *flip = match out_dir {
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
    }
}

const WEST_TO_EAST: TileTextureIndex = TileTextureIndex(11);
const SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(12);
const SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(13);
const NORTH_AND_SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(14);
const NORTH_AND_SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(15);

fn get_hover_tile(direction: SquareDirection) -> (TileTextureIndex, TileFlip) {
    get_conveyor_tile(opposite(direction), direction)
}

fn get_conveyor_tile(from: SquareDirection, to: SquareDirection) -> (TileTextureIndex, TileFlip) {
    use SquareDirection::*;

    match (from, to) {
        // straights
        (West, East) | (East, East) => (
            WEST_TO_EAST,
            TileFlip {
                x: false,
                y: false,
                d: false,
            },
        ),
        (East, West) | (West, West) => (
            WEST_TO_EAST,
            TileFlip {
                x: true,
                y: false,
                d: false,
            },
        ),
        (North, South) | (South, South) => (
            WEST_TO_EAST,
            TileFlip {
                x: false,
                y: false,
                d: true,
            },
        ),
        (South, North) | (North, North) => (
            WEST_TO_EAST,
            TileFlip {
                x: false,
                y: true,
                d: true,
            },
        ),

        // corners
        (East, North) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                y: true,
                ..default()
            },
        ),
        (East, South) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                ..default()
            },
        ),
        (North, East) => (
            SOUTH_TO_EAST,
            TileFlip {
                y: true,
                ..default()
            },
        ),
        (North, West) => (
            SOUTH_TO_EAST,
            TileFlip {
                x: true,
                y: true,
                ..default()
            },
        ),
        (West, North) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                x: true,
                y: true,
            },
        ),
        (West, South) => (
            SOUTH_TO_EAST,
            TileFlip {
                d: true,
                x: true,
                ..default()
            },
        ),
        (South, East) => (SOUTH_TO_EAST, TileFlip::default()),
        (South, West) => (
            SOUTH_TO_EAST,
            TileFlip {
                x: true,
                ..default()
            },
        ),

        (NorthEast, _)
        | (NorthWest, _)
        | (SouthWest, _)
        | (SouthEast, _)
        | (_, NorthEast)
        | (_, NorthWest)
        | (_, SouthWest)
        | (_, SouthEast) => panic!(),
    }
}

#[derive(Component, Clone, Debug, Reflect, Default)]
struct Conveyor(ConveyorDirection);

#[derive(Component, Reflect)]
struct HoveredTile(Option<ConveyorDirection>);

impl HoveredTile {
    fn set_to_next_option(&mut self) {
        use ConveyorDirection::*;

        self.0 = match self.0 {
            None => Some(East),
            Some(East) => Some(South),
            Some(South) => Some(West),
            Some(West) => Some(North),
            Some(North) => None,
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
