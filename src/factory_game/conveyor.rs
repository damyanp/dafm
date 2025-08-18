use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, BaseLayerEntityDespawned, ConveyorSystems,
        helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query, opposite},
    },
};

pub struct PayloadPlugin;
impl Plugin for PayloadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AcceptsPayloadConveyor>()
            .register_type::<BridgeConveyor>()
            .register_type::<Conveyor>()
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
                    update_bridge_conveyor_accepts_payload.in_set(ConveyorSystems::TileUpdater),
                    (
                        update_conveyor_inputs,
                        update_bridge_payloads,
                        transfer_payloads,
                        transfer_bridge_payloads,
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
    inputs: ConveyorDirections,
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
            inputs: ConveyorDirections::default(),
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
#[derive(Component, Default)]
pub struct CustomConveyorTransfer;

#[derive(Component, Debug, Reflect, Default)]
pub struct DistributorConveyor {
    pub next_output: ConveyorDirection,
}

#[derive(Component, Default, Reflect)]
#[require(CustomConveyorTransfer)]
pub struct BridgeConveyor {
    top: Vec<Entity>,
    bottom: Vec<Entity>,
}

/// Conveyors that accept input.
#[derive(Component, Default, Reflect, Debug)]
pub struct AcceptsPayloadConveyor(ConveyorDirections);

impl AcceptsPayloadConveyor {
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
pub struct SimpleConveyor;

#[derive(Component, Debug, Reflect)]
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

/// Bridge conveyors need to look at which neighbors are set to output to this
/// one to figure out where their inputs are.
fn update_bridge_conveyor_accepts_payload(
    new_conveyors: Query<&TilePos, Added<Conveyor>>,
    removed_entities: EventReader<BaseLayerEntityDespawned>,
    mut accepts_payloads: Query<&mut AcceptsPayloadConveyor, With<BridgeConveyor>>,
    conveyors: Query<&Conveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    let to_check = find_tiles_to_check(new_conveyors, removed_entities, map_size);

    for pos in to_check {
        if let Some(entity) = tile_storage.get(&pos)
            && let Ok(mut accepts) = accepts_payloads.get_mut(entity)
        {
            let neighbor_conveyors =
                get_neighbors_from_query(tile_storage, &pos, map_size, &conveyors);

            let inputs =
                neighbor_conveyors
                    .iter_with_direction()
                    .filter_map(|(direction, conveyor)| {
                        if conveyor.outputs().is_set(opposite(direction).into()) {
                            Some(ConveyorDirection::from(direction))
                        } else {
                            None
                        }
                    });

            *accepts = AcceptsPayloadConveyor(ConveyorDirections::from(inputs));
        }
    }
}

pub fn find_tiles_to_check(
    new: Query<&TilePos, Added<Conveyor>>,
    mut removed: EventReader<BaseLayerEntityDespawned>,
    map_size: &TilemapSize,
) -> HashSet<TilePos> {
    let mut to_check = HashSet::new();

    new.iter().for_each(|pos| {
        to_check.insert(*pos);
    });

    removed.read().for_each(|entity| {
        to_check.insert(entity.0);
    });

    let sources: Vec<_> = to_check.iter().cloned().collect();
    for pos in sources {
        for neighbor_pos in
            Neighbors::get_square_neighboring_positions(&pos, map_size, false).iter()
        {
            to_check.insert(*neighbor_pos);
        }
    }

    to_check
}

fn update_conveyor_inputs(
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
        if conveyor.inputs.is_none() {
            continue;
        }
        const MAX_PAYLOADS: usize = 1;

        let current_payload_count = payloads.map(|p| p.0.len()).unwrap_or(0);

        for payload in incoming
            .iter()
            .take(MAX_PAYLOADS.max(current_payload_count))
        {
            take_payload(
                commands.reborrow(),
                payload,
                receiver,
                &payload_destinations,
            );
        }
    }
}

fn update_bridge_payloads(
    bridges: Query<(Entity, &mut BridgeConveyor)>,
    payloads: Query<&PayloadOf>,
) {
    for (bridge_entity, mut bridge) in bridges {
        // TODO: This polls every bridge....which isn't great...but probably not
        // really a problem right now.

        let is_on_bridge = |entity: &Entity| {
            payloads
                .get(*entity)
                .map(|payload_of| payload_of.0 == bridge_entity)
                .unwrap_or(false)
        };

        bridge.top.retain(is_on_bridge);
        bridge.bottom.retain(is_on_bridge);
    }
}

fn transfer_bridge_payloads(
    mut commands: Commands,
    receivers: Query<(Entity, &PayloadsAwaitingTransfer, &mut BridgeConveyor)>,
    payload_destinations: Query<&PayloadDestination, With<PayloadAwaitingTransferTo>>,
) {
    for (receiver, incoming, mut bridge) in receivers {
        for payload in incoming.iter() {
            if let Ok(PayloadDestination(destination)) = payload_destinations.get(payload) {
                use ConveyorDirection::*;
                let take = match destination {
                    North | South => {
                        if bridge.bottom.is_empty() {
                            bridge.bottom.push(payload);
                            true
                        } else {
                            false
                        }
                    }
                    East | West => {
                        if bridge.top.is_empty() {
                            bridge.top.push(payload);
                            true
                        } else {
                            false
                        }
                    }
                };

                if take {
                    take_payload(
                        commands.reborrow(),
                        payload,
                        receiver,
                        &payload_destinations,
                    );
                }
            }
        }
    }
}

fn take_payload(
    mut commands: Commands,
    payload: Entity,
    receiver: Entity,
    payload_destinations: &Query<&PayloadDestination, With<PayloadAwaitingTransferTo>>,
) {
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

            if let Some(PayloadDestination(direction)) = destination
                && let Ok((_, conveyor_pos)) = conveyors.get(payload_of.0)
            {
                let destination_pos = conveyor_pos.square_offset(&(*direction).into(), map_size);
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
