use crate::GameState;
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::{
    helpers::square_grid::{
        SquarePos,
        neighbors::{CARDINAL_SQUARE_DIRECTIONS, Neighbors, SquareDirection},
    },
    prelude::*,
};
use bevy_egui::input::{egui_wants_any_keyboard_input, egui_wants_any_pointer_input};
use std::ops::DerefMut;

pub struct ConveyorPlugin;

mod helpers;
use helpers::*;

mod interaction;
use interaction::InteractionPlugin;
use interaction::*;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InteractionPlugin)
            .register_type::<Conveyor>()
            .insert_resource(MapConfig::default())
            .add_systems(OnEnter(GameState::Conveyor), startup)
            .add_systems(
                Update,
                ((
                    on_toggle_show_conveyors.run_if(input_just_pressed(KeyCode::Tab)),
                    on_test_data.run_if(input_just_pressed(KeyCode::KeyT)),
                )
                    .run_if(not(egui_wants_any_keyboard_input))
                    .run_if(not(egui_wants_any_pointer_input)),)
                    .chain()
                    .run_if(in_state(GameState::Conveyor)),
            )
            .add_observer(update_conveyor_tiles);
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<MapConfig>) {
    let texture = asset_server.load("sprites.png");
    commands.spawn(make_base_layer(&config, texture.to_owned()));
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

#[derive(Component, Clone, Debug, Reflect, Default)]
struct Conveyor(ConveyorDirection);

#[derive(Component)]
struct BaseLayer;

fn make_base_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (BaseLayer, make_layer(config, texture, 0.0, "BaseLayer"))
}

fn make_layer(config: &MapConfig, texture: Handle<Image>, z: f32, name: &'static str) -> impl Bundle {
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
            transform: Transform::from_xyz(0.0, 0.0, z),
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
