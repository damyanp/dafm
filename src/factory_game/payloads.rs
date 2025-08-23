use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use smallvec::SmallVec;

use crate::{
    factory_game::{BaseLayer, ConveyorSystems, conveyor::Conveyor, helpers::ConveyorDirection},
    helpers::TilemapQuery,
};

pub fn payloads_plugin(app: &mut App) {
    app.register_type::<Payload>()
        .register_type::<Payloads>()
        .register_type::<PayloadTransport>()
        .register_type::<PayloadTransportLine>()
        .add_event::<RequestPayloadTransferEvent>()
        .add_systems(
            Update,
            (
                (
                    transfer_payloads_to_transport_lines,
                    transfer_payloads_standard,
                )
                    .in_set(ConveyorSystems::TransferPayloads),
                (update_payload_mus, update_payload_transport_lines)
                    .in_set(ConveyorSystems::TransportLogic),
            ),
        )
        .add_systems(
            Update,
            (
                update_payload_transforms,
                update_payload_transport_line_transforms,
            )
                .in_set(ConveyorSystems::PayloadTransforms),
        );
}

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = Payload, linked_spawn)]
#[component(storage = "SparseSet")]
pub struct Payloads(Vec<Entity>);

#[derive(Component, Debug, Reflect)]
pub struct PayloadTransportLine {
    payloads: SmallVec<[(Entity, f32); 2]>,
    source: ConveyorDirection,
    destination: ConveyorDirection,
    spacing: f32,
}

impl PayloadTransportLine {
    pub fn new(destination: ConveyorDirection, spacing: f32) -> Self {
        PayloadTransportLine {
            payloads: SmallVec::default(),
            source: destination.opposite(),
            destination,
            spacing,
        }
    }

    fn try_transfer(&mut self, payload: Entity) {
        if self.has_room_for_one_more() {
            self.payloads.push((payload, 0.0));
        }
    }

    fn has_room_for_one_more(&self) -> bool {
        self.payloads
            .last()
            .map(|(_, mu)| *mu >= self.spacing)
            .unwrap_or(true)
    }

    fn update_payloads(&mut self, t: f32) {
        assert!(
            self.payloads.iter().all(|(_, mu)| *mu >= 0.0 && *mu <= 1.0),
            "All payload mu's in range 0 <= mu <= 1"
        );
        assert!(
            self.payloads.is_sorted_by_key(|(_, mu)| -mu),
            "Payloads are sorted by descending mu."
        );

        let mut last_mu = None;
        for (_, mu) in self.payloads.iter_mut() {
            let max_mu: f32 = last_mu.map(|mu| mu - self.spacing).unwrap_or(1.0);
            *mu = max_mu.min(*mu + t);
            last_mu = Some(*mu);
        }
    }
}

#[cfg(test)]
mod payload_transport_line_test {
    use super::*;

    #[test]
    fn empty() {
        let ptl = PayloadTransportLine::new(ConveyorDirection::East, 0.5);
        assert!(ptl.payloads.is_empty());
    }

    #[test]
    fn transfer_to_empty() {
        let mut ptl = PayloadTransportLine::new(ConveyorDirection::East, 0.5);
        let e = Entity::from_raw(1);
        ptl.try_transfer(e);
        assert_eq!(ptl.payloads.as_slice(), &[(e, 0.0)]);
    }

    #[test]
    fn transfer_doesnt_happen_when_no_room() {
        let mut ptl = PayloadTransportLine::new(ConveyorDirection::East, 0.5);
        let e1 = Entity::from_raw(1);
        ptl.try_transfer(e1);
        let e2 = Entity::from_raw(2);
        ptl.try_transfer(e2);

        assert_eq!(ptl.payloads.as_slice(), &[(e1, 0.0)]);
    }

    #[test]
    fn updates() {
        let mut ptl = PayloadTransportLine::new(ConveyorDirection::East, 0.5);
        let e: Vec<Entity> = (1..4).map(|i| Entity::from_raw(i)).collect();

        ptl.try_transfer(e[0]);
        ptl.try_transfer(e[1]);
        ptl.try_transfer(e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 0.1)]);

        ptl.try_transfer(e[1]);
        ptl.try_transfer(e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 0.2)]);

        ptl.try_transfer(e[1]);
        ptl.try_transfer(e[2]);
        ptl.update_payloads(0.3);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 0.5)]);

        ptl.try_transfer(e[1]);
        ptl.try_transfer(e[2]);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 0.5), (e[1], 0.0)]);

        ptl.update_payloads(0.5);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 1.0), (e[1], 0.5)]);

        ptl.try_transfer(e[2]);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[(e[0], 1.0), (e[1], 0.5), (e[2], 0.0)]
        );

        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[(e[0], 1.0), (e[1], 0.5), (e[2], 0.0)]
        );

        // Payloads bunch up - so if we remove one in the middle then the last
        // one will slide up as close as it is allowed to
        ptl.payloads.remove(1);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 1.0), (e[2], 0.5)]);
    }

    #[test]
    fn updates_with_different_spacing() {
        let mut ptl = PayloadTransportLine::new(ConveyorDirection::East, 0.2);
        let e: Vec<Entity> = (1..4).map(|i| Entity::from_raw(i)).collect();

        ptl.try_transfer(e[0]);
        ptl.try_transfer(e[1]);
        ptl.try_transfer(e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 0.1)]);

        ptl.try_transfer(e[1]);
        ptl.try_transfer(e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 0.2)]);

        ptl.try_transfer(e[1]);
        ptl.try_transfer(e[2]);
        ptl.update_payloads(0.3);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 0.5), (e[1], 0.3)]);

        ptl.update_payloads(0.5);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 1.0), (e[1], 0.8)]);

        ptl.try_transfer(e[2]);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[(e[0], 1.0), (e[1], 0.8), (e[2], 0.0)]
        );

        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[(e[0], 1.0), (e[1], 0.8), (e[2], 0.5)]
        );

        // Payloads bunch up - so if we remove one in the middle then the last
        // one will slide up as close as it is allowed to
        ptl.payloads.remove(1);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        assert_eq!(ptl.payloads.as_slice(), &[(e[0], 1.0), (e[2], 0.8)]);
    }
}

fn transfer_payloads_to_transport_lines(
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    mut transports: Query<&mut PayloadTransportLine>,
) {
    for e in transfers.read() {
        if let Ok(mut transport) = transports.get_mut(e.destination) {
            assert_eq!(e.direction, transport.source.opposite());
            transport.try_transfer(e.payload);
        }
    }
}

fn update_payload_transport_line_transforms() {}

fn update_payload_transport_lines(
    transport_lines: Query<&mut PayloadTransportLine>,
    time: Res<Time>,
) {
    let t = time.delta_secs();
    for mut transport_line in transport_lines {
        transport_line.update_payloads(t);
    }
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Payloads)]
#[component(storage = "SparseSet")]
pub struct Payload(pub Entity);

#[derive(Component, Reflect, Debug, Default)]
#[component(storage = "SparseSet")]
pub struct PayloadTransport {
    pub mu: f32,
    pub source: Option<ConveyorDirection>,
    pub destination: Option<ConveyorDirection>,
}

pub fn update_payload_mus(
    time: Res<Time>,
    payloads: Query<(Entity, &mut PayloadTransport, &Payload)>,
    conveyors: Query<(Entity, &TilePos)>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for (entity, mut payload, payload_of) in payloads {
        payload.mu += time.delta_secs() * 5.0;
        if payload.mu > 0.5 && payload.destination.is_none() {
            payload.mu = 0.5;
        } else if payload.mu > 1.0 {
            payload.mu = 1.0;

            if let Some(direction) = payload.destination
                && let Ok((_, conveyor_pos)) = conveyors.get(payload_of.0)
            {
                let destination_pos = conveyor_pos.square_offset(&direction.into(), map_size);
                let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));
                if let Some(destination_entity) = destination_entity {
                    let e = RequestPayloadTransferEvent {
                        payload: entity,
                        destination: destination_entity,
                        direction,
                    };
                    send_payloads.write(e);
                }
            }
        }
    }
}

pub fn update_payload_transforms(
    conveyors: Query<(&TilePos, &Payloads), With<Conveyor>>,
    mut payloads: Query<(&PayloadTransport, &mut Transform)>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, conveyor_payloads) in conveyors {
        let tile_center = base.center_in_world(tile_pos);

        for payload_entity in conveyor_payloads.iter() {
            let (payload, mut transform) = payloads.get_mut(payload_entity).unwrap();

            let start = tile_center + get_direction_offset(base.tile_size, payload.source);
            let end = tile_center + get_direction_offset(base.tile_size, payload.destination);

            let pos = if payload.mu < 0.5 {
                start.lerp(tile_center, payload.mu / 0.5)
            } else {
                tile_center.lerp(end, (payload.mu - 0.5) / 0.5)
            };

            let z = payload.destination.map(|d| {
                if d == ConveyorDirection::North || d == ConveyorDirection::South {
                    1.0
                } else {
                    3.0
                }
            });

            *transform = Transform::from_translation(pos.extend(z.unwrap_or(3.0)));
        }
    }
}

fn get_direction_offset(tile_size: &TilemapTileSize, direction: Option<ConveyorDirection>) -> Vec2 {
    let half_size = Vec2::new(tile_size.x / 2.0, tile_size.y / 2.0);

    match direction {
        Some(ConveyorDirection::North) => Vec2::new(0.0, half_size.y),
        Some(ConveyorDirection::South) => Vec2::new(0.0, -half_size.y),
        Some(ConveyorDirection::East) => Vec2::new(half_size.x, 0.0),
        Some(ConveyorDirection::West) => Vec2::new(-half_size.x, 0.0),
        None => Vec2::default(),
    }
}

#[derive(Event, Debug)]
pub struct RequestPayloadTransferEvent {
    pub payload: Entity,
    pub destination: Entity,
    pub direction: ConveyorDirection,
}

#[derive(Component, Default)]
pub struct SimpleConveyorTransferPolicy;

pub fn transfer_payloads_standard(
    mut commands: Commands,
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    receivers: Query<(&Conveyor, Option<&Payloads>), With<SimpleConveyorTransferPolicy>>,
) {
    for RequestPayloadTransferEvent {
        payload,
        destination,
        direction,
    } in transfers.read()
    {
        if let Ok((conveyor, payloads)) = receivers.get(*destination) {
            if conveyor.inputs().is_none() {
                continue;
            }
            const MAX_PAYLOADS: usize = 1;

            let current_payload_count = payloads.map(|p| p.len()).unwrap_or(0);

            if current_payload_count < MAX_PAYLOADS {
                commands.entity(*payload).insert((
                    Payload(*destination),
                    PayloadTransport {
                        source: Some(direction.opposite()),
                        destination: conveyor.single_or_no_output(),
                        ..default()
                    },
                ));
            }
        }
    }
}
