use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{Conveyor, PayloadOf, PayloadTransport, Payloads},
        helpers::{ConveyorDirection, ConveyorDirections},
    },
};

pub struct GeneratorPlugin;
impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_generator_tiles.in_set(ConveyorSystems::TileUpdater),
                generate_payloads.in_set(ConveyorSystems::TransportLogic),
            ),
        );
    }
}

#[derive(Component, Default, Debug)]
#[require(Conveyor = new_generator_conveyor())]
pub struct Generator {
    next_generate_time: f32,
}

fn new_generator_conveyor() -> Conveyor {
    Conveyor {
        outputs: ConveyorDirections::all(),
        accepts_input: false,
        is_full: true,
        next_output: ConveyorDirection::North,
    }
}

fn update_generator_tiles(
    mut commands: Commands,
    new_generators: Query<Entity, (With<Generator>, Without<TileTextureIndex>)>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TileStorage>)>,
) {
    for new_generator in new_generators {
        commands.entity(new_generator).insert_if_new((TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: TileTextureIndex(30),
            ..default()
        },));
    }
}

fn generate_payloads(
    mut commands: Commands,
    time: Res<Time>,
    generators: Query<(Entity, &mut Generator, Option<&Payloads>)>,
) {
    for (entity, mut generator, payloads) in generators {
        if time.elapsed_secs() > generator.next_generate_time && payloads.is_none() {
            commands.spawn((
                StateScoped(GameState::FactoryGame),
                Name::new("Payload"),
                PayloadOf(entity),
                Text2d::new("X"),
                TextColor(Color::linear_rgb(1.0, 0.4, 0.4)),
                PayloadTransport {
                    mu: 0.5,
                    source: None,
                    destination: None,
                },
            ));

            generator.next_generate_time = time.elapsed_secs() + 5.0;
        }
    }
}
