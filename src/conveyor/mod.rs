use crate::GameState;
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::{
    helpers::square_grid::{SquarePos, neighbors::CARDINAL_SQUARE_DIRECTIONS},
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

mod visuals;
use visuals::*;

impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InteractionPlugin)
            .add_plugins(Visuals)
            .register_type::<Conveyor>()
            .insert_resource(MapConfig::default())
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
            );
    }
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

#[derive(Component, Clone, Debug, Reflect, Default)]
struct Conveyor(ConveyorDirection);

#[derive(Event)]
struct ConveyorChanged(TilePos);

fn make_layer(
    config: &MapConfig,
    texture: Handle<Image>,
    z: f32,
    name: &'static str,
) -> impl Bundle {
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
