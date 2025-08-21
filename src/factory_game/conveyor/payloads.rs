use super::*;

/// Conveyors that accept input.
#[derive(Component, Default, Reflect, Debug)]
pub struct AcceptsPayloadConveyor(ConveyorDirections);

impl AcceptsPayloadConveyor {
    pub fn new(directions: ConveyorDirections) -> Self {
        AcceptsPayloadConveyor(directions)
    }

    pub fn all() -> Self {
        AcceptsPayloadConveyor(ConveyorDirections::all())
    }

    pub fn except(directions: ConveyorDirections) -> Self {
        AcceptsPayloadConveyor(ConveyorDirections::all_except(directions))
    }

    pub fn from_direction_iter(iter: impl Iterator<Item = ConveyorDirection>) -> Self {
        AcceptsPayloadConveyor(ConveyorDirections::from(iter))
    }
}

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = Payload, linked_spawn)]
#[component(storage = "SparseSet")]
pub struct Payloads(Vec<Entity>);

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

pub fn update_conveyor_inputs(
    conveyors: Query<(&mut Conveyor, &AcceptsPayloadConveyor, Option<&Payloads>)>,
) {
    for (mut conveyor, accepts_payload, payloads) in conveyors {
        let payload_count = payloads.map_or(0, |p| p.len());
        if payload_count == 0 {
            conveyor.inputs = accepts_payload.0;
        } else {
            conveyor.inputs = ConveyorDirections::default();
        }
    }
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

#[allow(clippy::type_complexity)]
pub fn update_payload_transforms(
    conveyors: Query<(&TilePos, &Payloads), With<Conveyor>>,
    mut payloads: Query<(&PayloadTransport, &mut Transform)>,
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

    for (tile_pos, conveyor_payloads) in conveyors {
        let tile_center =
            tile_pos.center_in_world(map_size, grid_size, tile_size, map_type, anchor);

        for payload_entity in conveyor_payloads.iter() {
            let (payload, mut transform) = payloads.get_mut(payload_entity).unwrap();

            let start = tile_center + get_direction_offset(tile_size, payload.source);
            let end = tile_center + get_direction_offset(tile_size, payload.destination);

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
