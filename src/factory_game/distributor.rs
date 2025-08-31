use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use smallvec::SmallVec;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        helpers::{ConveyorDirection, ConveyorDirections},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payload_handler::{AddPayloadHandler, PayloadHandler},
        payload_outputs::{PayloadOutputs, update_payload_handler_transforms},
        payloads::{Payload, PayloadTransportLine, RequestPayloadTransferEvent},
    },
    sprite_sheet::GameSprite,
};

pub fn distributor_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceDistributorEvent>()
        .add_payload_handler::<Distributor>()
        .add_systems(
            Update,
            (
                update_distributor_payloads.in_set(ConveyorSystems::TransportLogic),
                update_distributor_tiles.in_set(ConveyorSystems::TileUpdater),
                update_payload_handler_transforms::<Distributor>.in_set(ConveyorSystems::PayloadTransforms),
            ),
        );
}

pub struct DistributorTool(ConveyorDirection);
impl Default for DistributorTool {
    fn default() -> Self {
        DistributorTool(ConveyorDirection::East)
    }
}

impl Tool for DistributorTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::ToolDistributor, self.0.tile_flip())
    }

    fn next_variant(&mut self) {
        self.0 = self.0.next();
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceDistributorEvent(*tile_pos, self.0));
    }
}

#[derive(Event, Debug)]
pub struct PlaceDistributorEvent(TilePos, ConveyorDirection);

impl PlaceTileEvent for PlaceDistributorEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        let input_direction = self.1.opposite();
        let mut conveyor = Conveyor::default();
        let inputs = ConveyorDirections::new(input_direction);
        conveyor.set_inputs(inputs);
        conveyor.set_outputs(ConveyorDirections::all_except(inputs));

        commands.insert((
            Distributor::new(input_direction, 5),
            conveyor,
            Name::new("Distributor"),
        ));
    }
}

#[derive(Component, Debug, Reflect)]
pub struct Distributor {
    next_output: ConveyorDirection,
    input: PayloadTransportLine,
    outputs: SmallVec<[(ConveyorDirection, PayloadTransportLine); 3]>,
    capacity: u32,
}

impl PayloadHandler for Distributor {
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
        self.remove_payload_from_outputs(payload);
    }

    fn iter_payloads(&self) -> impl Iterator<Item = Entity> {
        self.input.iter_payloads().chain(self.iter_output_payloads())
    }
}

impl PayloadOutputs for Distributor {
    fn update_payloads(&mut self, t: f32) {
        self.input.update_payloads(t);
        self.outputs
            .iter_mut()
            .for_each(|(_, ptl)| ptl.update_payloads(t));
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

    fn update_all_payload_transforms(
        &self,
        tile_pos: &TilePos,
        payloads: &mut Query<&mut Transform, With<Payload>>,
        base: &crate::helpers::TilemapQueryItem,
    ) {
        self.input.update_payload_transforms(tile_pos, payloads, base);
        for (_, ptl) in &self.outputs {
            ptl.update_payload_transforms(tile_pos, payloads, base);
        }
    }

    fn remove_payload_from_outputs(&mut self, payload: Entity) {
        self.outputs
            .iter_mut()
            .for_each(|(_, ptl)| ptl.remove_payload(payload));
    }

    fn iter_output_payloads(&self) -> Box<dyn Iterator<Item = Entity> + '_> {
        Box::new(self.outputs.iter().flat_map(|(_, line)| line.iter_payloads()))
    }
}

impl Distributor {
    pub fn new(input: ConveyorDirection, capacity: u32) -> Self {
        let outputs = ConveyorDirections::all_except(ConveyorDirections::new(input));
        let outputs: SmallVec<_> = outputs
            .iter()
            .map(|dir| (dir, PayloadTransportLine::new(dir, capacity)))
            .collect();

        Self {
            next_output: ConveyorDirection::default(),
            input: PayloadTransportLine::new_no_output(capacity),
            outputs,
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
}

fn update_distributor_payloads(
    distributors: Query<(Entity, &mut Distributor, &TilePos)>,
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
    new_distributors: Query<(Entity, &Conveyor), Added<Distributor>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TilemapSize>)>,
) {
    for (e, conveyor) in new_distributors {
        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: GameSprite::Distributor.tile_texture_index(),
            flip: conveyor.input().opposite().tile_flip(),
            ..default()
        });
    }
}
