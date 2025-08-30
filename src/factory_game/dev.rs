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
    },
    helpers::TilemapQuery,
    sprite_sheet::{GameSprite, SpriteSheet},
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
    base: Single<TilemapQuery, With<BaseLayer>>,
    conveyors: Query<(&Conveyor, &TilePos)>,
    sprite_sheet: Res<SpriteSheet>,
    mut enabled: Local<bool>,
) {
    *enabled = !*enabled;

    if *enabled {
        for (conveyor, tile_pos) in conveyors {
            for output in conveyor.outputs().iter() {
                let angle = match output {
                    ConveyorDirection::North => std::f32::consts::FRAC_PI_2,
                    ConveyorDirection::South => -std::f32::consts::FRAC_PI_2,
                    ConveyorDirection::East => 0.0,
                    ConveyorDirection::West => std::f32::consts::PI,
                };
                let tile_center = base.center_in_world(tile_pos);

                commands.spawn((
                    StateScoped(GameState::FactoryGame),
                    Name::new("ConveyorDirection"),
                    DirectionArrow,
                    sprite_sheet.sprite(GameSprite::Arrow),
                    Transform::from_translation(tile_center.extend(5.0))
                        .with_rotation(Quat::from_rotation_z(angle)),
                ));
            }
        }
    } else {
        arrows
            .iter()
            .for_each(|arrow| commands.entity(arrow).despawn());
    }
}
