use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        distributor::{DistributePayloadEvent, DistributorConveyor},
        helpers::ConveyorDirections,
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        operators::{Operand, OperandPayloadBundle},
        payloads::{Payload, PayloadTransport, Payloads},
    },
    sprite_sheet::GameSprite,
};

pub fn generator_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceGeneratorEvent>()
        .register_type::<Generator>()
        .add_systems(
            Update,
            (
                update_generator_tiles.in_set(ConveyorSystems::TileUpdater),
                generate_payloads.in_set(ConveyorSystems::TransferPayloads),
            ),
        );
}

pub struct GeneratorTool;

impl Tool for GeneratorTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Generator, TileFlip::default())
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceGeneratorEvent(*tile_pos));
    }
}

#[derive(Event, Debug)]
pub struct PlaceGeneratorEvent(pub TilePos);

impl PlaceTileEvent for PlaceGeneratorEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((GeneratorBundle::new(), Name::new("Generator")));
    }
}

#[derive(Component, Debug, Reflect)]
struct Generator {
    next_generate_time: f32,
    time_between_generations: f32,
}

impl Default for Generator {
    fn default() -> Self {
        Generator {
            next_generate_time: 0.0,
            time_between_generations: 1.0,
        }
    }
}

#[derive(Bundle)]
pub struct GeneratorBundle {
    generator: Generator,
    conveyor: Conveyor,
    distributor: DistributorConveyor,
}

impl GeneratorBundle {
    pub fn new() -> Self {
        GeneratorBundle {
            generator: Generator::default(),
            conveyor: Conveyor::new(ConveyorDirections::all()),
            distributor: DistributorConveyor::default(),
        }
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
            texture_index: GameSprite::Generator.tile_texture_index(),
            ..default()
        },));
    }
}

fn generate_payloads(
    mut commands: Commands,
    time: Res<Time>,
    generators: Query<(Entity, &mut Generator, Option<&Payloads>)>,
    mut events: EventWriter<DistributePayloadEvent>,
) {
    for (entity, mut generator, payloads) in generators {
        if time.elapsed_secs() > generator.next_generate_time && payloads.is_none() {
            let payload = commands
                .spawn((
                    OperandPayloadBundle::new(Operand(1)),
                    Payload(entity),
                    PayloadTransport {
                        mu: 0.5,
                        ..default()
                    },
                ))
                .id();
            events.write(DistributePayloadEvent {
                transporter: entity,
                payload,
            });
            generator.next_generate_time = time.elapsed_secs() + generator.time_between_generations;
        }
    }
}
