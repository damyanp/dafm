use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    prelude::*,
};

use crate::factory_game::{
    BaseLayer, BaseLayerEntityDespawned, ConveyorSystems,
    helpers::{
        ConveyorDirection, ConveyorDirections, get_neighbors_from_query, make_east_relative,
        opposite,
    },
    payload::{OfferPayloadEvent, PayloadOf, PayloadTransport, Payloads, TookPayloadEvent},
};

pub struct ConveyorPlugin;
impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Conveyor>().add_systems(
            Update,
            (
                update_conveyor_tiles.in_set(ConveyorSystems::TileUpdater),
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

#[derive(Component)]
pub struct ConveyorBelt;

fn update_conveyor_tiles(
    mut commands: Commands,
    new_conveyors: Query<&TilePos, (With<Conveyor>, Without<TileTextureIndex>)>,
    mut removed_entities: EventReader<BaseLayerEntityDespawned>,
    conveyors: Query<(&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>)>,
    base: Single<(Entity, &TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tilemap_entity, tile_storage, map_size) = base.into_inner();

    let mut to_check = HashSet::new();

    new_conveyors.iter().for_each(|pos| {
        to_check.insert(*pos);
    });

    removed_entities.read().for_each(|entity| {
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

    for pos in to_check {
        if let Some(entity) = tile_storage.get(&pos) {
            if let Ok(conveyor) = conveyors.get(entity) {
                commands.entity(entity).insert_if_new(TileBundle {
                    tilemap_id: TilemapId(tilemap_entity),
                    ..default()
                });

                update_conveyor_belt_tile(
                    commands.reborrow(),
                    entity,
                    conveyor,
                    &pos,
                    tile_storage,
                    map_size,
                    &conveyors,
                );
            }
        }
    }
}

fn update_conveyor_belt_tile(
    mut commands: Commands,
    entity: Entity,
    conveyor: (&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>),
    tile_pos: &TilePos,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    conveyors: &Query<(&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>)>,
) {
    let (Conveyor(out_dir), texture_index, flip) = conveyor;

    let out_dir: SquareDirection = (out_dir.single()).into();

    // Find the neighbors that have conveyors on them
    let neighbor_conveyors = get_neighbors_from_query(tile_storage, tile_pos, map_size, conveyors);

    // And just the conveyors pointing towards this one
    let neighbor_conveyors = Neighbors::from_directional_closure(|dir| {
        neighbor_conveyors.get(dir).and_then(|(c, _, _)| {
            let directions = c.0;

            if directions.is_set(opposite(dir).into()) {
                Some(*c)
            } else {
                None
            }
        })
    });

    // Rotate all of this so that east is always the "out" direction
    let neighbor_conveyors = make_east_relative(neighbor_conveyors, out_dir);

    let (new_texture_index, y_flip) = match neighbor_conveyors {
        Neighbors {
            north: None,
            east: None,
            south: None,
            west: Some(_),
            ..
        } => (WEST_TO_EAST, false),
        Neighbors {
            north: None,
            east: None,
            south: Some(_),
            west: Some(_),
            ..
        } => (SOUTH_AND_WEST_TO_EAST, false),
        Neighbors {
            north: Some(_),
            east: None,
            south: None,
            west: Some(_),
            ..
        } => (SOUTH_AND_WEST_TO_EAST, true),
        Neighbors {
            north: None,
            east: None,
            south: Some(_),
            west: None,
            ..
        } => (SOUTH_TO_EAST, false),
        Neighbors {
            north: Some(_),
            east: None,
            south: None,
            west: None,
            ..
        } => (SOUTH_TO_EAST, true),
        Neighbors {
            north: Some(_),
            east: None,
            south: Some(_),
            west: None,
            ..
        } => (NORTH_AND_SOUTH_TO_EAST, false),
        Neighbors {
            north: Some(_),
            east: None,
            south: Some(_),
            west: Some(_),
            ..
        } => (NORTH_AND_SOUTH_AND_WEST_TO_EAST, false),
        _ => (WEST_TO_EAST, false),
    };

    // y_flip indicates if we should flip y for the "east is always out"
    // orientation.  Now we need to rotate the tile so that the out
    // direction is correct.  For North/South this means that y_flip
    // actually becomes an x_flip.
    let new_flip = match out_dir {
        SquareDirection::North => TileFlip {
            x: y_flip,
            y: true,
            d: true,
        },
        SquareDirection::South => TileFlip {
            x: !y_flip,
            y: false,
            d: true,
        },
        SquareDirection::East => TileFlip {
            x: false,
            y: y_flip,
            d: false,
        },
        SquareDirection::West => TileFlip {
            x: true,
            y: !y_flip,
            d: false,
        },
        _ => panic!(),
    };

    if Some(&new_texture_index) != texture_index || Some(&new_flip) != flip {
        commands
            .entity(entity)
            .insert((new_texture_index, new_flip));
    }
}

const WEST_TO_EAST: TileTextureIndex = TileTextureIndex(11);
const SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(12);
const SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(13);
const NORTH_AND_SOUTH_TO_EAST: TileTextureIndex = TileTextureIndex(14);
const NORTH_AND_SOUTH_AND_WEST_TO_EAST: TileTextureIndex = TileTextureIndex(15);

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
