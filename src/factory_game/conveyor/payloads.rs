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
#[relationship_target(relationship = PayloadOf, linked_spawn)]
#[component(storage = "SparseSet")]
pub struct Payloads(Vec<Entity>);

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Payloads)]
#[require(PayloadTransport)]
#[component(storage = "SparseSet")]
pub struct PayloadOf(pub Entity);

#[derive(Component, Reflect, Debug, Default)]
#[component(storage = "SparseSet")]
pub struct PayloadTransport {
    pub mu: f32,
}

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
pub struct PayloadSource(pub ConveyorDirection);

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
pub struct PayloadDestination(pub ConveyorDirection);

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

pub fn take_payload(
    mut commands: Commands,
    payload: Entity,
    receiver: Entity,
    destination: Option<&PayloadDestination>,
) {
    if let Some(PayloadDestination(direction)) = destination {
        commands
            .entity(payload)
            .remove::<(PayloadOf, PayloadTransport, PayloadDestination)>()
            .insert((PayloadOf(receiver), PayloadSource(direction.opposite())));
    } else {
        println!("* {payload:?} doesn't have a destination set");
    }
}

pub fn update_payload_mus(
    time: Res<Time>,
    payloads: Query<(
        Entity,
        &mut PayloadTransport,
        &PayloadOf,
        Option<&PayloadDestination>,
    )>,
    conveyors: Query<(Entity, &TilePos)>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for (entity, mut payload, payload_of, destination) in payloads {
        payload.mu += time.delta_secs() * 5.0;
        if payload.mu > 0.5 && destination.is_none() {
            payload.mu = 0.5;
        } else if payload.mu > 1.0 {
            payload.mu = 1.0;

            if let Some(PayloadDestination(direction)) = destination
                && let Ok((_, conveyor_pos)) = conveyors.get(payload_of.0)
            {
                let destination_pos = conveyor_pos.square_offset(&(*direction).into(), map_size);
                let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));
                if let Some(destination_entity) = destination_entity {
                    send_payloads.write(RequestPayloadTransferEvent {
                        payload: entity,
                        destination: destination_entity,
                    });
                }
            }
        }
    }
}

pub fn update_simple_conveyor_destinations(
    mut commands: Commands,
    simple_conveyors: Query<(&Conveyor, &Payloads), With<SimpleConveyor>>,
    payloads: Query<Entity, (With<PayloadTransport>, Without<PayloadDestination>)>,
) {
    for (conveyor, conveyor_payloads) in simple_conveyors {
        let direction = conveyor.outputs.single();
        for payload in payloads.iter_many(conveyor_payloads.0.iter()) {
            commands
                .entity(payload)
                .insert(PayloadDestination(direction));
        }
    }
}

pub fn update_distributor_conveyor_destinations(
    mut commands: Commands,
    mut distributor_conveyors: Query<(&TilePos, &Conveyor, &Payloads, &mut DistributorConveyor)>,
    payloads: Query<Entity, (With<PayloadTransport>, Without<PayloadDestination>)>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    conveyors: Query<&Conveyor>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for (tile_pos, conveyor, conveyor_payloads, mut distributor_conveyor) in
        distributor_conveyors.iter_mut()
    {
        let neighbors = get_neighbors_from_query(tile_storage, tile_pos, map_size, &conveyors);
        for payload in payloads.iter_many(conveyor_payloads.0.iter()) {
            let direction = conveyor
                .outputs
                .iter_from(distributor_conveyor.next_output)
                .find(|direction| {
                    let neighbor = neighbors.get((*direction).into());
                    neighbor
                        .map(|conveyor| conveyor.inputs.is_set(direction.opposite()))
                        .unwrap_or(false)
                });
            if let Some(direction) = direction {
                commands
                    .entity(payload)
                    .insert(PayloadDestination(direction));
                distributor_conveyor.next_output = direction.next();
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update_payload_transforms(
    conveyors: Query<(&TilePos, &Payloads), With<Conveyor>>,
    mut payloads: Query<(
        &PayloadTransport,
        Option<&PayloadSource>,
        Option<&PayloadDestination>,
        &mut Transform,
    )>,
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
            let (payload_transport, source, destination, mut transform) =
                payloads.get_mut(payload_entity).unwrap();

            let start = tile_center + get_direction_offset(tile_size, source.map(|s| s.0));
            let end = tile_center + get_direction_offset(tile_size, destination.map(|d| d.0));

            let pos = if payload_transport.mu < 0.5 {
                start.lerp(tile_center, payload_transport.mu / 0.5)
            } else {
                tile_center.lerp(end, (payload_transport.mu - 0.5) / 0.5)
            };

            let z = destination.map(|d| {
                if d.0 == ConveyorDirection::North || d.0 == ConveyorDirection::South {
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
