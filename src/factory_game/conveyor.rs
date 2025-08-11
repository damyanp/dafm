use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::SquareDirection, prelude::*};

use crate::factory_game::{
    BaseLayer, ConveyorSystems,
    helpers::{ConveyorDirection, ConveyorDirections, opposite},
};

pub struct PayloadPlugin;
impl Plugin for PayloadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Conveyor>()
            .register_type::<PayloadOf>()
            .register_type::<Payloads>()
            .register_type::<PayloadTransport>()
            .add_event::<OfferPayloadEvent>()
            .add_systems(
                Update,
                (
                    (take_payloads, update_is_full, transport_conveyor_payloads)
                        .chain()
                        .in_set(ConveyorSystems::TransportLogic),
                    update_conveyor_payloads.in_set(ConveyorSystems::PayloadTransforms),
                ),
            );
    }
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Conveyor {
    pub outputs: ConveyorDirections,
    pub accepts_input: bool,
    pub next_output: ConveyorDirection,
    pub is_full: bool,
}

impl Conveyor {
    pub fn new_belt(direction: ConveyorDirection) -> Self {
        Conveyor {
            outputs: ConveyorDirections::new(direction),
            accepts_input: true,
            next_output: direction,
            is_full: false,
        }
    }
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Payloads)]
pub struct PayloadOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = PayloadOf, linked_spawn)]
pub struct Payloads(Vec<Entity>);

#[derive(Component, Reflect)]
pub struct PayloadTransport {
    pub mu: f32,
    pub source: Option<ConveyorDirection>,
    pub destination: Option<ConveyorDirection>,
}

#[derive(Event)]
pub struct OfferPayloadEvent {
    pub source_direction: ConveyorDirection,
    pub payload: Entity,
    pub target: Entity,
}

fn take_payloads(
    mut commands: Commands,
    mut offer_events: EventReader<OfferPayloadEvent>,
    conveyors: Query<&Conveyor>,
) {
    // Only accept one offer per-conveyer per-update (since we can't easily
    // requery between events)
    let mut conveyors_accepted = HashSet::new();

    for offer in offer_events.read() {
        if !conveyors_accepted.contains(&offer.target)
            && let Ok(conveyor) = conveyors.get(offer.target)
        {
            if conveyor.accepts_input && !conveyor.is_full {
                commands.entity(offer.payload).insert((
                    PayloadOf(offer.target),
                    PayloadTransport {
                        mu: 0.0,
                        source: Some(offer.source_direction),
                        destination: None,
                    },
                ));
                conveyors_accepted.insert(offer.target);
            }
        }
    }
}

fn update_is_full(conveyors: Query<(&mut Conveyor, Option<&Payloads>)>) {
    for (mut conveyor, payloads) in conveyors {
        if conveyor.accepts_input {
            let payload_count = payloads.map_or(0, |p| p.len());
            conveyor.is_full = payload_count > 0;
        }
    }
}

fn transport_conveyor_payloads(
    time: Res<Time>,
    mut payload_transports: Query<&mut PayloadTransport>,
    conveyors: Query<(Entity, &TilePos, &Payloads), With<Conveyor>>,
    mut mutable_conveyors: Query<&mut Conveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut offer_payload_event: EventWriter<OfferPayloadEvent>,
) {
    let mu_speed = time.delta_secs();

    let (tile_storage, map_size) = base.into_inner();

    for (conveyor_entity, conveyor_pos, payloads) in conveyors {
        for payload_entity in payloads.iter() {
            if let Ok(mut transport) = payload_transports.get_mut(payload_entity) {
                transport.mu += mu_speed;

                if transport.mu > 0.5 {
                    if let Some(destination) = transport.destination {
                        let destination = destination.into();
                        let destination_pos = conveyor_pos.square_offset(&destination, map_size);
                        let destination_entity =
                            destination_pos.and_then(|pos| tile_storage.get(&pos));

                        if transport.mu > 1.0 {
                            transport.mu = 1.0;
                            if let Some(destination_entity) = destination_entity {
                                offer_payload_event.write(OfferPayloadEvent {
                                    source_direction: opposite(destination).into(),
                                    payload: payload_entity,
                                    target: destination_entity,
                                });
                            }
                        }
                    } else {
                        // no destination - pick one!
                        transport.destination = pick_next_destination(
                            tile_storage,
                            map_size,
                            conveyor_pos,
                            conveyor_entity,
                            mutable_conveyors.as_readonly(),
                        );

                        if let Some(destination) = transport.destination {
                            mutable_conveyors
                                .get_mut(conveyor_entity)
                                .unwrap()
                                .next_output = destination.next();
                        } else {
                            transport.mu = 0.5;
                        }
                    }
                }
            }
        }
    }
}

fn pick_next_destination(
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    conveyor_pos: &TilePos,
    conveyor_entity: Entity,
    conveyors: Query<&Conveyor>,
) -> Option<ConveyorDirection> {
    let conveyor = conveyors.get(conveyor_entity);

    conveyor.ok().and_then(|conveyor| {
        conveyor
            .outputs
            .iter_from(conveyor.next_output)
            .find(|direction| {
                let direction: SquareDirection = (*direction).into();

                let neighbor_pos = conveyor_pos.square_offset(&direction, map_size);
                let neighbor_entity = neighbor_pos.and_then(|p| tile_storage.get(&p));
                let neighbor_conveyor = neighbor_entity
                    .and_then(|e| conveyors.get(e).map(|c| c.accepts_input && !c.is_full).ok());
                neighbor_conveyor.unwrap_or(false)
            })
    })
}

#[allow(clippy::type_complexity)]
fn update_conveyor_payloads(
    conveyors: Query<(&TilePos, &Payloads), With<Conveyor>>,
    mut payloads: Query<(Option<&PayloadTransport>, &mut Transform), With<PayloadOf>>,
    base: Single<
        (
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<BaseLayer>,
    >,
) {
    let (map_size, grid_size, tile_size, map_type, anchor) = base.into_inner();

    for (tile_pos, generator_payloads) in conveyors {
        let tile_center =
            tile_pos.center_in_world(map_size, grid_size, tile_size, map_type, anchor);

        for payload_entity in generator_payloads.iter() {
            let (transport, mut transform) = payloads.get_mut(payload_entity).unwrap();

            let pos = if let Some(transport) = transport {
                let start = tile_center + get_direction_offset(tile_size, &transport.source);
                let end = tile_center + get_direction_offset(tile_size, &transport.destination);

                if transport.mu < 0.5 {
                    start.lerp(tile_center, transport.mu / 0.5)
                } else {
                    tile_center.lerp(end, (transport.mu - 0.5) / 0.5)
                }
            } else {
                tile_center
            };
            *transform = Transform::from_translation(pos.extend(2.0));
        }
    }
}

fn get_direction_offset(
    tile_size: &TilemapTileSize,
    direction: &Option<ConveyorDirection>,
) -> Vec2 {
    let half_size = Vec2::new(tile_size.x / 2.0, tile_size.y / 2.0);

    match direction {
        Some(ConveyorDirection::North) => Vec2::new(0.0, half_size.y),
        Some(ConveyorDirection::South) => Vec2::new(0.0, -half_size.y),
        Some(ConveyorDirection::East) => Vec2::new(half_size.x, 0.0),
        Some(ConveyorDirection::West) => Vec2::new(-half_size.x, 0.0),
        None => Vec2::default(),
    }
}
