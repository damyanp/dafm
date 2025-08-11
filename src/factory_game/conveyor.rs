use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

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
            .add_event::<TookPayloadEvent>()
            .add_systems(
                Update,
                (
                    (take_payloads, transport_conveyor_payloads)
                        .chain()
                        .in_set(ConveyorSystems::TransportLogic),
                    update_conveyor_payloads.in_set(ConveyorSystems::PayloadTransforms),
                ),
            );
    }
}

#[derive(Component, Clone, Debug, Reflect, Default)]
pub struct Conveyor(pub ConveyorDirections);

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Payloads)]
pub struct PayloadOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = PayloadOf, linked_spawn)]
pub struct Payloads(Vec<Entity>);

#[derive(Component, Reflect)]
pub struct PayloadTransport {
    pub mu: f32,
    pub source: ConveyorDirection,
    pub destination: ConveyorDirection,
}

#[derive(Event)]
pub struct OfferPayloadEvent {
    pub source_direction: ConveyorDirection,
    pub payload: Entity,
    pub target: Entity,
}

#[derive(Event)]
pub struct TookPayloadEvent {
    pub payload: Entity,
}

fn take_payloads(
    mut commands: Commands,
    mut offer_events: EventReader<OfferPayloadEvent>,
    mut took_events: EventWriter<TookPayloadEvent>,
    conveyors: Query<(&Conveyor, Option<&Payloads>)>,
) {
    // Only accept one offer per-conveyer per-update (since we can't easily
    // requery between events)
    let mut conveyors_accepted = HashSet::new();

    for offer in offer_events.read() {
        if !conveyors_accepted.contains(&offer.target)
            && let Ok((conveyor, payloads)) = conveyors.get(offer.target)
        {
            let payload_count = payloads.map_or(0, |p| p.len());
            if payload_count == 0 {
                commands.entity(offer.payload).insert((
                    PayloadOf(offer.target),
                    PayloadTransport {
                        mu: 0.0,
                        source: offer.source_direction,
                        destination: conveyor.0.single(),
                    },
                ));
                took_events.write(TookPayloadEvent {
                    payload: offer.payload,
                });
                conveyors_accepted.insert(offer.target);
            }
        }
    }
}

fn transport_conveyor_payloads(
    time: Res<Time>,
    mut payload_transports: Query<&mut PayloadTransport>,
    conveyors: Query<(&TilePos, &Payloads), With<Conveyor>>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut offer_payload_event: EventWriter<OfferPayloadEvent>,
) {
    let mu_speed = time.delta_secs();

    let (tile_storage, map_size) = base.into_inner();

    for (conveyor_pos, payloads) in conveyors {
        for payload_entity in payloads.iter() {
            if let Ok(mut transport) = payload_transports.get_mut(payload_entity) {
                let destination_pos =
                    conveyor_pos.square_offset(&transport.destination.into(), map_size);
                let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));

                transport.mu += mu_speed;
                if transport.mu > 1.0 {
                    transport.mu = 1.0;
                    if let Some(destination_entity) = destination_entity {
                        offer_payload_event.write(OfferPayloadEvent {
                            source_direction: opposite(transport.destination.into()).into(),
                            payload: payload_entity,
                            target: destination_entity,
                        });
                    }
                }
            }
        }
    }
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

fn get_direction_offset(tile_size: &TilemapTileSize, direction: &ConveyorDirection) -> Vec2 {
    let half_size = Vec2::new(tile_size.x / 2.0, tile_size.y / 2.0);

    match direction {
        ConveyorDirection::North => Vec2::new(0.0, half_size.y),
        ConveyorDirection::South => Vec2::new(0.0, -half_size.y),
        ConveyorDirection::East => Vec2::new(half_size.x, 0.0),
        ConveyorDirection::West => Vec2::new(-half_size.x, 0.0),
    }
}
