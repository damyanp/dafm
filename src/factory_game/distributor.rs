use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use smallvec::SmallVec;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{Conveyor, TilesToCheck},
        conveyor_belts::find_incoming_directions,
        helpers::{ConveyorDirection, ConveyorDirections},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payload_handler::{AddPayloadHandler, PayloadHandler},
        payloads::{Payload, PayloadTransportLine, RequestPayloadTransferEvent},
    },
    helpers::TilemapQuery,
    sprite_sheet::GameSprite,
};

pub fn distributor_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceDistributorEvent>()
        .add_payload_handler::<DistributorConveyor>()
        .add_systems(
            Update,
            (
                update_distributor_payloads.in_set(ConveyorSystems::TransportLogic),
                (update_distributor_conveyors, update_distributor_tiles)
                    .in_set(ConveyorSystems::TileUpdater),
                update_distributor_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
            ),
        );
}

pub struct DistributorTool;
impl Tool for DistributorTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Distributor, TileFlip::default())
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceDistributorEvent(*tile_pos));
    }
}

#[derive(Event, Debug)]
pub struct PlaceDistributorEvent(TilePos);

impl PlaceTileEvent for PlaceDistributorEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((Distributor, Name::new("Distributor")));
    }
}

#[derive(Component)]
#[require(Conveyor::new(ConveyorDirections::all()), DistributorConveyor::new(5))]
struct Distributor;

#[derive(Component, Debug, Reflect)]
pub struct DistributorConveyor {
    next_output: ConveyorDirection,
    input: PayloadTransportLine,
    outputs: SmallVec<[(ConveyorDirection, PayloadTransportLine); 3]>,
    capacity: u32,
}

impl PayloadHandler for DistributorConveyor {
    fn try_transfer(
        &mut self,
        self_conveyor: &Conveyor,
        request: &RequestPayloadTransferEvent,
    ) -> Option<Entity> {
        if self.count() >= self.capacity as usize {
            return None;
        }

        self.input.try_transfer(self_conveyor, request)
    }

    fn remove_payload(&mut self, payload: Entity) {
        self.outputs
            .iter_mut()
            .for_each(|(_, ptl)| ptl.remove_payload(payload));
    }

    fn iter_payloads(&self) -> impl Iterator<Item = Entity> {
        self.input.iter_payloads().chain(
            self.outputs
                .iter()
                .flat_map(|(_, line)| line.iter_payloads()),
        )
    }
}

impl DistributorConveyor {
    pub fn new(capacity: u32) -> Self {
        Self {
            next_output: ConveyorDirection::default(),
            input: PayloadTransportLine::new_no_output(capacity),
            outputs: SmallVec::default(),
            capacity,
        }
    }

    fn count(&self) -> usize {
        self.input.count()
            + self
                .outputs
                .iter()
                .map(|(_, o)| o.count())
                .reduce(|a, b| a + b)
                .unwrap_or(0)
    }

    fn update_payloads(&mut self, t: f32) {
        self.input.update_payloads(t);
        self.outputs
            .iter_mut()
            .for_each(|(_, ptl)| ptl.update_payloads(t));
    }

    pub fn distribute<F>(
        &mut self,
        self_conveyor: &Conveyor,
        tile_storage: &TileStorage,
        tile_pos: &TilePos,
        map_size: &TilemapSize,
        conveyors: &Query<&Conveyor>,
        get_payload: F,
    ) -> Option<Entity>
    where
        F: FnOnce() -> Entity,
    {
        if let Some(destination) = self_conveyor.get_available_destination(
            self.next_output,
            tile_storage,
            tile_pos,
            map_size,
            conveyors,
        ) {
            let payload = self
                .outputs
                .iter_mut()
                .find(|(dir, _)| *dir == destination)
                .and_then(|(_, ptl)| {
                    ptl.try_transfer_onto_with_mu(ConveyorDirection::default(), 0.5, get_payload)
                });

            if let Some(payload) = payload {
                self.input.remove_payload(payload);
            }

            self.next_output = destination.next();

            return payload;
        }
        None
    }

    fn get_payload_to_transfer(&self) -> Option<(ConveyorDirection, Entity)> {
        for (dir, output) in &self.outputs {
            let p = output.get_payload_to_transfer().map(|e| (*dir, e));
            if p.is_some() {
                return p;
            }
        }
        None
    }
}

fn update_distributor_conveyors(
    mut commands: Commands,
    to_check: Res<TilesToCheck>,
    mut conveyors: Query<&mut Conveyor>,
    mut distributors: Query<&mut DistributorConveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for tile_pos in &to_check.0 {
        if let Some(entity) = tile_storage.get(tile_pos)
            && let Ok(mut distributor) = distributors.get_mut(entity)
        {
            let inputs = find_incoming_directions(
                tile_pos,
                tile_storage,
                map_size,
                &conveyors.as_readonly(),
            );

            let outputs = ConveyorDirections::all_except(inputs);

            if let Ok(mut conveyor) = conveyors.get_mut(entity) {
                conveyor.set_inputs(inputs);
                conveyor.set_outputs(outputs);

                // Drop any output transport lines - and despawn any entities on them
                distributor.outputs.retain(|(dir, ptl)| {
                    if outputs.is_set(*dir) {
                        return true;
                    }

                    ptl.despawn_payloads(commands.reborrow());
                    false
                });

                // Add any new output lines
                outputs.iter().for_each(|dir| {
                    if distributor.outputs.iter().all(|(d, _)| *d != dir) {
                        let ptl = PayloadTransportLine::new(dir, distributor.capacity);
                        distributor.outputs.push((dir, ptl));
                    }
                });
            }
        }
    }
}

fn update_distributor_payloads(
    distributors: Query<(Entity, &mut DistributorConveyor, &TilePos)>,
    conveyors: Query<&Conveyor>,
    time: Res<Time>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();
    let t = time.delta_secs();

    for (source, mut distributor, tile_pos) in distributors {
        distributor.update_payloads(t);

        if let Ok(conveyor) = conveyors.get(source) {
            if let Some(payload) = distributor.input.get_payload_to_transfer() {
                distributor.distribute(
                    conveyor,
                    tile_storage,
                    tile_pos,
                    map_size,
                    &conveyors,
                    || payload,
                );
            }

            if let Some((dir, payload)) = distributor.get_payload_to_transfer() {
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
}

fn update_distributor_tiles(
    mut commands: Commands,
    new_distributors: Query<Entity, Added<Distributor>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TilemapSize>)>,
) {
    for e in new_distributors {
        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: GameSprite::Distributor.tile_texture_index(),
            ..default()
        });
    }
}

fn update_distributor_payload_transforms(
    distributors: Query<(&TilePos, &DistributorConveyor)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, distributor) in distributors {
        distributor
            .input
            .update_payload_transforms(tile_pos, &mut payloads, &base);
        for (_, ptl) in &distributor.outputs {
            ptl.update_payload_transforms(tile_pos, &mut payloads, &base);
        }
    }
}
