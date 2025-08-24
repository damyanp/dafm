use bevy::{ecs::event::EventCursor, prelude::*};
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        conveyor::{AcceptsPayloadConveyor, Conveyor}, helpers::{get_neighbors_from_query, ConveyorDirection, ConveyorDirections}, interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool}, payloads::{Payload, PayloadTransferredEvent, PayloadTransport, Payloads, RequestPayloadTransferEvent}, BaseLayer, ConveyorSystems
    },
    sprite_sheet::GameSprite,
};

pub fn distributor_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceDistributorEvent>()
        .add_event::<DistributePayloadEvent>()
        .register_type::<DistributorConveyor>()
        .add_systems(
            Update,
            (
                transfer_payloads_to_distributors.in_set(ConveyorSystems::TransferPayloads),
                distribute_payloads.in_set(ConveyorSystems::TransportLogic),
                update_distributor_tiles.in_set(ConveyorSystems::TileUpdater),
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
#[require(
    Conveyor::new(ConveyorDirections::all()),
    DistributorConveyor::default(),
    AcceptsPayloadConveyor::all()
)]
struct Distributor;

#[derive(Component, Debug, Reflect, Default)]
pub struct DistributorConveyor {
    pub next_output: ConveyorDirection,
}

fn transfer_payloads_to_distributors(
    mut commands: Commands,
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    mut receivers: Query<Option<&Payloads>, With<DistributorConveyor>>,
    mut events: EventWriter<DistributePayloadEvent>,
    mut transferred: EventWriter<PayloadTransferredEvent>,
) {
    for RequestPayloadTransferEvent {
        payload,
        source,
        destination,
        direction,
    } in transfers.read()
    {
        if let Ok(payloads) = receivers.get_mut(*destination) {
            const MAX_PAYLOADS: usize = 1;

            let current_payload_count = payloads.map(|p| p.len()).unwrap_or(0);

            if current_payload_count < MAX_PAYLOADS {
                commands.entity(*payload).insert((
                    Payload(*destination),
                    PayloadTransport {
                        source: Some(direction.opposite()),
                        ..default()
                    },
                ));
                events.write(DistributePayloadEvent {
                    transporter: *destination,
                    payload: *payload,
                });
                transferred.write(PayloadTransferredEvent {
                    payload: *payload,
                    source: *source,
                });
            }
        }
    }
}

#[derive(Event)]
pub struct DistributePayloadEvent {
    pub transporter: Entity,
    pub payload: Entity,
}

fn distribute_payloads(
    mut events: ResMut<Events<DistributePayloadEvent>>,
    mut reader: Local<EventCursor<DistributePayloadEvent>>,
    mut distributors: Query<(&Conveyor, &TilePos, &mut DistributorConveyor)>,
    mut payloads: Query<&mut PayloadTransport>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    conveyors: Query<&Conveyor>,
) {
    let (tile_storage, map_size) = base.into_inner();

    let mut retry_events = Vec::new();

    for DistributePayloadEvent {
        transporter,
        payload,
    } in reader.read(&events)
    {
        if let Ok((conveyor, tile_pos, mut distributor)) = distributors.get_mut(*transporter)
            && let Ok(mut payload_transport) = payloads.get_mut(*payload)
        {
            // Figure out where this payload will be going
            let neighbors = get_neighbors_from_query(tile_storage, tile_pos, map_size, &conveyors);
            let destination_direction =
                conveyor
                    .outputs()
                    .iter_from(distributor.next_output)
                    .find(|direction| {
                        let neighbor = neighbors.get((*direction).into());
                        neighbor
                            .map(|conveyor| conveyor.inputs().is_set(direction.opposite()))
                            .unwrap_or(false)
                    });

            if let Some(destination_direction) = destination_direction {
                assert!(payload_transport.destination.is_none());
                payload_transport.destination = Some(destination_direction);
                distributor.next_output = destination_direction.next();
            } else {
                retry_events.push(DistributePayloadEvent {
                    transporter: *transporter,
                    payload: *payload,
                });
            }
        }
    }

    retry_events.into_iter().for_each(|event| {
        events.send(event);
    });
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
