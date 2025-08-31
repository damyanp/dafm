use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use smallvec::SmallVec;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{Conveyor, ConveyorUpdated, TilesToCheck},
        helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query},
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
                (update_generators, update_generator_tiles).in_set(ConveyorSystems::TileUpdater),
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
    outputs: SmallVec<[(ConveyorDirection, PayloadTransportLine); 4]>,
    next_output: ConveyorDirection,
}

impl Default for Generator {
    fn default() -> Self {
        Generator {
            next_generate_time: 0.0,
            time_between_generations: 1.0,
            outputs: SmallVec::default(),
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
        None
    }

    fn remove_payload(&mut self, payload: Entity) {
        self.outputs
            .iter_mut()
            .for_each(|(_, ptl)| ptl.remove_payload(payload));
    }

    fn iter_payloads(&self) -> impl Iterator<Item = Entity> {
        std::iter::empty().chain(self.outputs.iter().flat_map(|(_, ptl)| ptl.iter_payloads()))
    }
}

impl Generator {
    fn update_payloads(&mut self, t: f32) {
        self.outputs
            .iter_mut()
            .for_each(|(_, ptl)| ptl.update_payloads(t));
    }

    fn get_payload_to_transfer(&self) -> Option<(ConveyorDirection, Entity)> {
        for (dir, output) in self.outputs.iter() {
            let p = output.get_payload_to_transfer().map(|e| (*dir, e));
            if p.is_some() {
                return p;
            }
        }
        None
    }
}

fn update_generators(
    mut commands: Commands,
    to_check: Res<TilesToCheck>,
    mut generators: Query<&mut Generator>,
    mut conveyors: Query<&mut Conveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut conveyor_updated: EventWriter<ConveyorUpdated>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for tile_pos in &to_check.0 {
        if let Some(entity) = tile_storage.get(tile_pos)
            && let Ok(mut generator) = generators.get_mut(entity)
        {
            let neighbors = get_neighbors_from_query(tile_storage, tile_pos, map_size, &conveyors);

            let output_directions =
                ConveyorDirections::from(neighbors.iter_with_direction().filter_map(
                    |(dir, conveyor)| {
                        let dir = ConveyorDirection::from(dir);
                        if conveyor.outputs().is_set(dir.opposite()) {
                            None
                        } else {
                            Some(dir)
                        }
                    },
                ));

            if let Ok(mut conveyor) = conveyors.get_mut(entity) {
                let old_outputs = conveyor.outputs();
                conveyor.set_outputs(output_directions);

                if old_outputs != output_directions {
                    conveyor_updated.write(ConveyorUpdated(*tile_pos));
                }

                generator.outputs.retain_mut(|(dir, ptl)| {
                    if output_directions.is_set(*dir) {
                        true
                    } else {
                        ptl.despawn_payloads(commands.reborrow());
                        false
                    }
                });

                for direction in output_directions.iter() {
                    if generator.outputs.iter().all(|(dir, _)| *dir != direction) {
                        generator
                            .outputs
                            .push((direction, PayloadTransportLine::new(direction, 1)));
                    }
                }
            }
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
            if let Some((_, ptl)) = generator
                .outputs
                .iter_mut()
                .find(|(dir, _)| *dir == destination)
            {
                let payload =
                    ptl.try_transfer_onto_with_mu(ConveyorDirection::default(), 0.5, || {
                        commands.spawn(operand_bundle(Operand(1))).id()
                    });

                if payload.is_some() {
                    generator.next_generate_time =
                        time.elapsed_secs() + generator.time_between_generations;
                }
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
        for (_, ptl) in &generator.outputs {
            ptl.update_payload_transforms(tile_pos, &mut payloads, &base);
        }
    }
}
