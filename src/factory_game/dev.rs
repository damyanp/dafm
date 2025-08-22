use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::{
    helpers::square_grid::{SquarePos, neighbors::CARDINAL_SQUARE_DIRECTIONS},
    prelude::*,
};
use bevy_egui::input::{egui_wants_any_keyboard_input, egui_wants_any_pointer_input};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorDirection, conveyor::Conveyor, conveyor_belts::conveyor_belt_bundle,
        interaction::InteractionLayer,
    },
    sprite_sheet::GameSprite,
};

pub fn dev_plugin(app: &mut App) {
    app.add_systems(
        Update,
        ((
            on_toggle_show_conveyors.run_if(input_just_pressed(KeyCode::Tab)),
            on_test_data.run_if(input_just_pressed(KeyCode::KeyT)),
        )
            .run_if(not(egui_wants_any_keyboard_input))
            .run_if(not(egui_wants_any_pointer_input)),)
            .chain()
            .run_if(in_state(GameState::FactoryGame)),
    );
}

fn on_test_data(
    mut commands: Commands,
    base: Single<(&mut TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (mut storage, map_size) = base.into_inner();

    let mut pos = SquarePos { x: 32, y: 58 };

    let mut spawn = |pos: SquarePos, direction| {
        let pos = pos.as_tile_pos(map_size).unwrap();
        storage.set(
            &pos,
            commands
                .spawn((
                    StateScoped(GameState::FactoryGame),
                    Name::new("Test Data Tile"),
                    conveyor_belt_bundle(direction),
                    BaseLayer,
                    pos,
                ))
                .id(),
        );
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
            if conveyor.outputs().is_multiple() {
                continue;
            }
            let flip = match conveyor.output() {
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
                StateScoped(GameState::FactoryGame),
                Name::new("ConveyorDirection"),
                DirectionArrow,
                TileBundle {
                    texture_index: GameSprite::Arrow.tile_texture_index(),
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
