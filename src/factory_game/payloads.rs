use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use smallvec::SmallVec;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        helpers::ConveyorDirection,
        payload_handler::{AddPayloadHandler, PayloadHandler},
    },
    helpers::{TilemapQuery, TilemapQueryItem},
};

pub fn payloads_plugin(app: &mut App) {
    app.add_payload_handler::<PayloadTransportLine>()
        .add_event::<RequestPayloadTransferEvent>()
        .add_event::<PayloadTransferredEvent>()
        .add_systems(
            Update,
            ((update_payload_transport_lines).in_set(ConveyorSystems::TransportLogic),),
        )
        .add_systems(
            Update,
            (update_payload_transport_line_transforms,).in_set(ConveyorSystems::PayloadTransforms),
        )
        .add_observer(on_remove_payload_transport_line);
}

#[derive(Component, Debug, Reflect)]
pub struct PayloadTransportLine {
    payloads: SmallVec<[TransportedPayload; 2]>,
    output_direction: ConveyorDirection,
    capacity: u32,
}

#[derive(Debug, Reflect, PartialEq)]
struct TransportedPayload {
    entity: Entity,
    from: ConveyorDirection,
    mu: f32,
}

impl TransportedPayload {
    fn new(entity: Entity, from: ConveyorDirection, mu: f32) -> Self {
        TransportedPayload { entity, from, mu }
    }
}

impl PayloadHandler for PayloadTransportLine {
    fn try_transfer(&mut self, _: &Conveyor, request: &RequestPayloadTransferEvent) -> bool {
        self.try_transfer_onto(request.direction.opposite(), || request.payload)
    }

    fn remove_payload(&mut self, payload: Entity) {
        self.payloads.retain(|p| p.entity != payload);
    }
}

impl PayloadTransportLine {
    pub fn new(destination: ConveyorDirection, capacity: u32) -> Self {
        PayloadTransportLine {
            payloads: SmallVec::default(),
            output_direction: destination,
            capacity,
        }
    }

    pub fn output_direction(&self) -> ConveyorDirection {
        self.output_direction
    }

    pub fn try_transfer_onto<F>(&mut self, from: ConveyorDirection, get_payload: F) -> bool
    where
        F: FnOnce() -> Entity,
    {
        self.try_transfer_onto_with_mu(from, 0.0, get_payload)
    }

    pub fn try_transfer_onto_with_mu<F>(
        &mut self,
        from: ConveyorDirection,
        mu: f32,
        get_payload: F,
    ) -> bool
    where
        F: FnOnce() -> Entity,
    {
        if self.has_room_for_one_more_with_mu(mu) {
            self.payloads
                .push(TransportedPayload::new(get_payload(), from, mu));
            true
        } else {
            false
        }
    }

    fn has_room_for_one_more_with_mu(&self, mu: f32) -> bool {
        self.payloads
            .last()
            .map(|p| p.mu >= self.spacing() + mu)
            .unwrap_or(true)
    }

    fn spacing(&self) -> f32 {
        1.0 / (self.capacity as f32)
    }

    pub fn update(
        &mut self,
        this_entity: Entity,
        tile_pos: &TilePos,
        t: f32,
        tile_storage: &TileStorage,
        map_size: &TilemapSize,
        send_payloads: &mut EventWriter<RequestPayloadTransferEvent>,
    ) {
        self.update_payloads(t);
        if let Some(payload) = self.get_payload_to_transfer() {
            let destination_pos = tile_pos.square_offset(&self.output_direction.into(), map_size);
            let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));
            if let Some(destination) = destination_entity {
                let e = RequestPayloadTransferEvent {
                    payload,
                    source: this_entity,
                    destination,
                    direction: self.output_direction,
                };
                send_payloads.write(e);
            }
        }
    }

    fn update_payloads(&mut self, t: f32) {
        assert!(
            self.payloads.iter().all(|p| p.mu >= 0.0 && p.mu <= 1.0),
            "All payload mu's in range 0 <= mu <= 1"
        );
        assert!(
            self.payloads.is_sorted_by_key(|p| -p.mu),
            "Payloads are sorted by descending mu."
        );
        let spacing = self.spacing();

        let mut last_mu = None;
        for p in self.payloads.iter_mut() {
            let max_mu: f32 = last_mu.map(|mu| mu - spacing).unwrap_or(1.0);
            p.mu = max_mu.min(p.mu + t);
            last_mu = Some(p.mu);
        }
    }

    fn get_payload_to_transfer(&self) -> Option<Entity> {
        if let Some(p) = self.payloads.first()
            && p.mu == 1.0
        {
            return Some(p.entity);
        }

        None
    }

    pub fn update_payload_transforms(
        &self,
        tile_pos: &TilePos,
        payloads: &mut Query<&mut Transform, With<Payload>>,
        base: &TilemapQueryItem,
    ) {
        let tile_center = base.center_in_world(tile_pos);
        for p in &self.payloads {
            if let Ok(mut transform) = payloads.get_mut(p.entity) {
                *transform = get_payload_transform(
                    tile_center,
                    base.tile_size,
                    Some(p.from),
                    Some(self.output_direction),
                    p.mu,
                );
            }
        }
    }

    pub fn despawn_payloads(&self, mut commands: Commands) {
        for p in &self.payloads {
            commands.entity(p.entity).try_despawn();
        }
    }
}

#[cfg(test)]
mod payload_transport_line_test {
    use super::*;
    use ConveyorDirection::*;

    fn tp(entity: Entity, from: ConveyorDirection, mu: f32) -> TransportedPayload {
        TransportedPayload { entity, from, mu }
    }

    #[test]
    fn empty() {
        let ptl = PayloadTransportLine::new(East, 2);
        assert!(ptl.payloads.is_empty());
    }

    #[test]
    fn transfer_to_empty() {
        let mut ptl = PayloadTransportLine::new(East, 2);
        let e = Entity::from_raw(1);
        ptl.try_transfer_onto(West, || e);
        assert_eq!(ptl.payloads.as_slice(), &[tp(e, West, 0.0)]);
    }

    #[test]
    fn transfer_doesnt_happen_when_no_room() {
        let mut ptl = PayloadTransportLine::new(East, 2);
        let e1 = Entity::from_raw(1);
        ptl.try_transfer_onto(West, || e1);
        let e2 = Entity::from_raw(2);
        ptl.try_transfer_onto(West, || e2);

        assert_eq!(ptl.payloads.as_slice(), &[tp(e1, West, 0.0)]);
    }

    #[test]
    fn updates() {
        let mut ptl = PayloadTransportLine::new(ConveyorDirection::East, 2);
        let e: Vec<Entity> = (1..4).map(|i| Entity::from_raw(i)).collect();

        ptl.try_transfer_onto(West, || e[0]);
        ptl.try_transfer_onto(West, || e[1]);
        ptl.try_transfer_onto(West, || e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[tp(e[0], West, 0.1)]);

        ptl.try_transfer_onto(West, || e[1]);
        ptl.try_transfer_onto(West, || e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[tp(e[0], West, 0.2)]);

        ptl.try_transfer_onto(West, || e[1]);
        ptl.try_transfer_onto(West, || e[2]);
        ptl.update_payloads(0.3);
        assert_eq!(ptl.payloads.as_slice(), &[tp(e[0], West, 0.5)]);

        ptl.try_transfer_onto(West, || e[1]);
        ptl.try_transfer_onto(West, || e[2]);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[tp(e[0], West, 0.5), tp(e[1], West, 0.0)]
        );

        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[tp(e[0], West, 1.0), tp(e[1], West, 0.5)]
        );

        ptl.try_transfer_onto(West, || e[2]);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[
                tp(e[0], West, 1.0),
                tp(e[1], West, 0.5),
                tp(e[2], West, 0.0)
            ]
        );

        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[
                tp(e[0], West, 1.0),
                tp(e[1], West, 0.5),
                tp(e[2], West, 0.0)
            ]
        );

        // Payloads bunch up - so if we remove one in the middle then the last
        // one will slide up as close as it is allowed to
        ptl.payloads.remove(1);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[tp(e[0], West, 1.0), tp(e[2], West, 0.5)]
        );
    }

    #[test]
    fn updates_with_different_spacing() {
        let mut ptl = PayloadTransportLine::new(ConveyorDirection::East, 5);
        let e: Vec<Entity> = (1..4).map(|i| Entity::from_raw(i)).collect();

        ptl.try_transfer_onto(West, || e[0]);
        ptl.try_transfer_onto(West, || e[1]);
        ptl.try_transfer_onto(West, || e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[tp(e[0], West, 0.1)]);

        ptl.try_transfer_onto(West, || e[1]);
        ptl.try_transfer_onto(West, || e[2]);
        ptl.update_payloads(0.1);
        assert_eq!(ptl.payloads.as_slice(), &[tp(e[0], West, 0.2)]);

        ptl.try_transfer_onto(West, || e[1]);
        ptl.try_transfer_onto(West, || e[2]);
        ptl.update_payloads(0.3);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[tp(e[0], West, 0.5), tp(e[1], West, 0.3)]
        );

        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[tp(e[0], West, 1.0), tp(e[1], West, 0.8)]
        );

        ptl.try_transfer_onto(West, || e[2]);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[
                tp(e[0], West, 1.0),
                tp(e[1], West, 0.8),
                tp(e[2], West, 0.0)
            ]
        );

        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[
                tp(e[0], West, 1.0),
                tp(e[1], West, 0.8),
                tp(e[2], West, 0.5)
            ]
        );

        // Payloads bunch up - so if we remove one in the middle then the last
        // one will slide up as close as it is allowed to
        ptl.payloads.remove(1);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        ptl.update_payloads(0.5);
        assert_eq!(
            ptl.payloads.as_slice(),
            &[tp(e[0], West, 1.0), tp(e[2], West, 0.8)]
        );
    }
}

fn update_payload_transport_lines(
    transport_lines: Query<(Entity, &mut PayloadTransportLine, &TilePos)>,
    time: Res<Time>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();

    let t = time.delta_secs();
    for (source, mut transport_line, tile_pos) in transport_lines {
        transport_line.update(
            source,
            tile_pos,
            t,
            tile_storage,
            map_size,
            &mut send_payloads,
        );
    }
}

fn update_payload_transport_line_transforms(
    transport_lines: Query<(&TilePos, &PayloadTransportLine)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, transport) in transport_lines {
        transport.update_payload_transforms(tile_pos, &mut payloads, &base);
    }
}

fn on_remove_payload_transport_line(
    trigger: Trigger<OnRemove, PayloadTransportLine>,
    transports: Query<&PayloadTransportLine>,
    commands: Commands,
) {
    // despawn anything that this line was holding
    if let Ok(transport) = transports.get(trigger.target()) {
        transport.despawn_payloads(commands);
    }
}

#[derive(Component, Default)]
pub struct Payload;

pub fn get_payload_transform(
    tile_center: Vec2,
    tile_size: &TilemapTileSize,
    input_direction: Option<ConveyorDirection>,
    output_direction: Option<ConveyorDirection>,
    mu: f32,
) -> Transform {
    let start = tile_center + get_direction_offset(tile_size, input_direction);
    let end = tile_center + get_direction_offset(tile_size, output_direction);

    let pos = if mu < 0.5 {
        start.lerp(tile_center, mu / 0.5)
    } else {
        tile_center.lerp(end, (mu - 0.5) / 0.5)
    };

    let z = output_direction.map(|d| {
        if d == ConveyorDirection::North || d == ConveyorDirection::South {
            1.0
        } else {
            3.0
        }
    });

    Transform::from_translation(pos.extend(z.unwrap_or(3.0)))
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
    pub source: Entity,
    pub destination: Entity,
    pub direction: ConveyorDirection,
}

#[derive(Event, Debug)]
pub struct PayloadTransferredEvent {
    pub payload: Entity,
    pub source: Entity,
}
