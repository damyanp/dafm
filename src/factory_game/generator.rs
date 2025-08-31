use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        helpers::{CONVEYOR_DIRECTIONS, ConveyorDirection, ConveyorDirections},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        operators::{Operand, operand_bundle},
        payload_handler::{AddPayloadHandler, PayloadHandler},
        payloads::{Payload, PayloadTransportLine, RequestPayloadTransferEvent},
    },
    helpers::TilemapQuery,
    sprite_sheet::GameSprite,
};

pub fn generator_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceGeneratorEvent>()
        .add_payload_handler::<Generator>()
        .add_systems(
            Update,
            (
                update_generator_payloads.in_set(ConveyorSystems::TransportLogic),
                update_generator_tiles.in_set(ConveyorSystems::TileUpdater),
                generate_payloads.in_set(ConveyorSystems::TransferPayloadsToHandlers),
                update_generator_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
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
        commands.insert((Generator::default(), Name::new("Generator")));
    }
}

#[derive(Component, Debug, Reflect)]
#[require(Conveyor::new(ConveyorDirections::all()))]
struct Generator {
    next_generate_time: f32,
    time_between_generations: f32,
    outputs: [PayloadTransportLine; 4],
    next_output: ConveyorDirection,
}

impl Default for Generator {
    fn default() -> Self {
        Generator {
            next_generate_time: 0.0,
            time_between_generations: 1.0,
            outputs: CONVEYOR_DIRECTIONS.map(|d| PayloadTransportLine::new(d, 1)),
            next_output: ConveyorDirection::default(),
        }
    }
}

impl PayloadHandler for Generator {
    fn try_transfer(
        &mut self,
        _: &Conveyor,
        _: &super::payloads::RequestPayloadTransferEvent,
    ) -> Option<Entity> {
        panic!("Nothing should ever try to transfer to a Generator!");
    }

    fn remove_payload(&mut self, payload: Entity) {
        self.outputs
            .iter_mut()
            .for_each(|ptl| ptl.remove_payload(payload));
    }

    fn iter_payloads(&self) -> impl Iterator<Item = Entity> {
        std::iter::empty().chain(self.outputs.iter().flat_map(|ptl| ptl.iter_payloads()))
    }
}

impl Generator {
    fn update_payloads(&mut self, t: f32) {
        self.outputs
            .iter_mut()
            .for_each(|ptl| ptl.update_payloads(t));
    }

    fn get_payload_to_transfer(&self) -> Option<(ConveyorDirection, Entity)> {
        for (dir, output) in self.outputs.iter().enumerate() {
            let dir = ConveyorDirection::from(dir);
            let p = output.get_payload_to_transfer().map(|e| (dir, e));
            if p.is_some() {
                return p;
            }
        }
        None
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
    generators: Query<(&TilePos, &Conveyor, &mut Generator)>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    conveyors: Query<&Conveyor>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for (tile_pos, conveyor, mut generator) in generators {
        if time.elapsed_secs() > generator.next_generate_time
            && let Some(destination) = conveyor.get_available_destination(
                generator.next_output,
                tile_storage,
                tile_pos,
                map_size,
                &conveyors,
            )
        {
            let ptl = &mut generator.outputs[destination.index()];
            let payload = ptl.try_transfer_onto_with_mu(ConveyorDirection::default(), 0.5, || {
                commands.spawn(operand_bundle(Operand(1))).id()
            });

            if payload.is_some() {
                generator.next_generate_time =
                    time.elapsed_secs() + generator.time_between_generations;
            }

            generator.next_output = generator.next_output.next();
        }
    }
}

fn update_generator_payloads(
    generators: Query<(Entity, &mut Generator, &TilePos)>,
    time: Res<Time>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();
    let t = time.delta_secs();

    for (source, mut generator, tile_pos) in generators {
        generator.update_payloads(t);

        if let Some((dir, payload)) = generator.get_payload_to_transfer() {
            let destination_pos = tile_pos.square_offset(&dir.into(), map_size);
            let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));
            if let Some(destination) = destination_entity {
                let e = RequestPayloadTransferEvent {
                    payload,
                    source,
                    destination,
                    direction: dir,
                };
                send_payloads.write(e);
            }
        }
    }
}

fn update_generator_payload_transforms(
    generators: Query<(&TilePos, &Generator)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, generator) in generators {
        for ptl in &generator.outputs {
            ptl.update_payload_transforms(tile_pos, &mut payloads, &base);
        }
    }
}
