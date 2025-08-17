use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorSystems,
        helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query},
    },
};

pub struct PayloadPlugin;
impl Plugin for PayloadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Conveyor>()
            .register_type::<PayloadOf>()
            .register_type::<Payloads>()
            .register_type::<PayloadTransport>()
            .register_type::<PayloadDestination>()
            .register_type::<PayloadSource>()
            .register_type::<DistributorConveyor>()
            .register_type::<PayloadsAwaitingTransfer>()
            .register_type::<PayloadAwaitingTransferTo>()
            .add_systems(
                Update,
                (
                    (
                        update_accepts_input,
                        transfer_payloads,
                        update_payload_mus,
                        (
                            update_simple_conveyor_destinations,
                            update_distributor_conveyor_destinations,
                            update_bridge_conveyor_destinations,
                        ),
                    )
                        .chain()
                        .in_set(ConveyorSystems::TransportLogic),
                    update_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
                ),
            );
    }
}

#[derive(Component, Clone, Debug, Reflect, Default)]
#[require(StateScoped::<GameState>(GameState::FactoryGame))]
pub struct Conveyor {
    outputs: ConveyorDirections,
    accepts_input: bool,
}

impl From<ConveyorDirection> for Conveyor {
    fn from(direction: ConveyorDirection) -> Self {
        Conveyor::new(ConveyorDirections::new(direction))
    }
}

impl Conveyor {
    pub fn new(outputs: ConveyorDirections) -> Self {
        Conveyor {
            outputs,
            accepts_input: false,
        }
    }

    pub fn output(&self) -> ConveyorDirection {
        self.outputs.single()
    }

    pub fn outputs(&self) -> ConveyorDirections {
        self.outputs
    }
}

/// Prevents [`transfer_payloads`] from operating on this entity.
#[derive(Component)]
pub struct CustomConveyorTransfer;

#[derive(Component, Debug, Reflect, Default)]
pub struct DistributorConveyor {
    pub next_output: ConveyorDirection,
}

#[derive(Component)]
pub struct BridgeConveyor;

/// Conveyors that accept input.
#[derive(Component)]
pub struct AcceptsPayloadConveyor;

#[derive(Component, Debug, Reflect)]
pub struct SimpleConveyor;

#[derive(Component, Reflect)]
#[relationship_target(relationship = PayloadOf, linked_spawn)]
#[component(storage = "SparseSet")]
pub struct Payloads(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship_target(relationship = PayloadAwaitingTransferTo)]
#[component(storage = "SparseSet")]
pub struct PayloadsAwaitingTransfer(Vec<Entity>);

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

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = PayloadsAwaitingTransfer)]
#[component(storage = "SparseSet")]
pub struct PayloadAwaitingTransferTo(Entity);

fn update_accepts_input(
    conveyors: Query<(&mut Conveyor, Option<&Payloads>), With<AcceptsPayloadConveyor>>,
) {
    for (mut conveyor, payloads) in conveyors {
        let payload_count = payloads.map_or(0, |p| p.len());
        conveyor.accepts_input = payload_count == 0;
    }
}

fn transfer_payloads(
    mut commands: Commands,
    receivers: Query<
        (
            Entity,
            &Conveyor,
            &PayloadsAwaitingTransfer,
            Option<&Payloads>,
        ),
        Without<CustomConveyorTransfer>,
    >,
    payload_destinations: Query<&PayloadDestination, With<PayloadAwaitingTransferTo>>,
) {
    for (receiver, conveyor, incoming, payloads) in receivers {
        if !conveyor.accepts_input {
            continue;
        }
        const MAX_PAYLOADS: usize = 1;

        let current_payload_count = payloads.map(|p| p.0.len()).unwrap_or(0);

        for payload in incoming
            .iter()
            .take(MAX_PAYLOADS.max(current_payload_count))
        {
            if let Ok(direction) = payload_destinations.get(payload) {
                commands
                    .entity(payload)
                    .remove::<(
                        PayloadOf,
                        PayloadAwaitingTransferTo,
                        PayloadTransport,
                        PayloadDestination,
                    )>()
                    .insert((PayloadOf(receiver), PayloadSource(direction.0.opposite())));
            } else {
                println!("* {payload:?} doesn't have a destination set");
            }
        }
    }
}

fn update_payload_mus(
    mut commands: Commands,
    time: Res<Time>,
    payloads: Query<
        (
            Entity,
            &mut PayloadTransport,
            &PayloadOf,
            Option<&PayloadDestination>,
        ),
        Without<PayloadAwaitingTransferTo>,
    >,
    conveyors: Query<(Entity, &TilePos)>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for (entity, mut payload, payload_of, destination) in payloads {
        payload.mu += time.delta_secs();
        if payload.mu > 0.5 && destination.is_none() {
            payload.mu = 0.5;
        } else if payload.mu > 1.0 {
            payload.mu = 1.0;

            if let Some(PayloadDestination(direction)) = destination {
                if let Ok((_, conveyor_pos)) = conveyors.get(payload_of.0) {
                    let destination_pos =
                        conveyor_pos.square_offset(&(*direction).into(), map_size);
                    let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));
                    if let Some(destination_entity) = destination_entity {
                        commands
                            .entity(entity)
                            .insert(PayloadAwaitingTransferTo(destination_entity));
                    }
                }
            }
        }
    }
}

fn update_simple_conveyor_destinations(
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

fn update_distributor_conveyor_destinations(
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
                        .map(|conveyor| {
                            conveyor.accepts_input && !conveyor.outputs.is_set(direction.opposite())
                        })
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

#[expect(clippy::type_complexity)]
fn update_bridge_conveyor_destinations(
    mut commands: Commands,
    bridge_conveyors: Query<&Payloads, With<BridgeConveyor>>,
    payloads: Query<
        (Entity, &PayloadSource),
        (With<PayloadTransport>, Without<PayloadDestination>),
    >,
) {
    for bridge_payloads in bridge_conveyors {
        for (payload, source) in payloads.iter_many(bridge_payloads.iter()) {
            commands
                .entity(payload)
                .insert(PayloadDestination(source.0.opposite()));
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_payload_transforms(
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

            *transform = Transform::from_translation(pos.extend(2.0));
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
