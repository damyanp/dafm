use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    conveyor::{
        ConveyorSystems,
        conveyor::Conveyor,
        helpers::{CONVEYOR_DIRECTIONS, ConveyorDirection, get_neighbors_from_query},
        payload::{PayloadOf, PayloadTransport, Payloads},
        visuals::BaseLayer,
    },
};

pub struct GeneratorPlugin;
impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_generator_tiles.in_set(ConveyorSystems::TileUpdater),
        )
        .add_systems(
            Update,
            (
                (
                    generate_payloads,
                    set_generator_payload_transports,
                    transport_generator_payloads,
                )
                    .chain()
                    .in_set(ConveyorSystems::TransportLogic),
                update_generator_payloads.in_set(ConveyorSystems::PayloadTransforms),
            ),
        );
    }
}

#[derive(Component, Default, Debug)]
pub struct Generator {
    next_generate_time: f32,
    next_transport_direction_index: u32,
}

fn update_generator_tiles(
    mut commands: Commands,
    new_generators: Query<Entity, (With<Generator>, Without<TileTextureIndex>)>,
    mut removed_generators: RemovedComponents<Generator>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TileStorage>)>,
) {
    for new_generator in new_generators {
        commands.entity(new_generator).insert_if_new((TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: TileTextureIndex(30),
            ..default()
        },));
    }

    for removed_generator in removed_generators.read() {
        commands
            .entity(removed_generator)
            .remove::<TileTextureIndex>();
    }
}

fn generate_payloads(
    mut commands: Commands,
    time: Res<Time>,
    generators: Query<(Entity, &mut Generator, Option<&Payloads>)>,
) {
    for (entity, mut generator, payloads) in generators {
        if time.elapsed_secs() > generator.next_generate_time && payloads.is_none() {
            commands.spawn((
                StateScoped(GameState::Conveyor),
                Name::new("Payload"),
                PayloadOf(entity),
                Text2d::new("X"),
            ));

            generator.next_generate_time = time.elapsed_secs() + 5.0;
        }
    }
}

fn set_generator_payload_transports(
    mut commands: Commands,
    untransported_payloads: Query<(Entity, &PayloadOf), Without<PayloadTransport>>,
    mut generators: Query<(&TilePos, &mut Generator)>,
    conveyors: Query<Entity, With<Conveyor>>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for (untransported_entity, payload_of) in untransported_payloads {
        if let Ok((generator_pos, mut generator)) = generators.get_mut(payload_of.0) {
            let conveyor_neighbors =
                get_neighbors_from_query(tile_storage, generator_pos, map_size, &conveyors);

            for i in 0..CONVEYOR_DIRECTIONS.len() {
                let direction_index = (generator.next_transport_direction_index as usize + i)
                    % CONVEYOR_DIRECTIONS.len();
                let d = CONVEYOR_DIRECTIONS[direction_index];

                if conveyor_neighbors.get(d.into()).is_some() {
                    commands
                        .entity(untransported_entity)
                        .insert(PayloadTransport::new(d));
                    generator.next_transport_direction_index =
                        ((direction_index + 1) % CONVEYOR_DIRECTIONS.len()) as u32;
                    break;
                }
            }
        }
    }
}

fn transport_generator_payloads(
    mut commands: Commands,
    time: Res<Time>,
    mut payload_transports: Query<&mut PayloadTransport>,
    generators: Query<(&TilePos, &Payloads), With<Generator>>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let mu_speed = time.delta_secs();

    let (tile_storage, map_size) = base.into_inner();

    for (generator_pos, payloads) in generators {
        for payload_entity in payloads.iter() {
            if let Ok(mut transport) = payload_transports.get_mut(payload_entity) {
                let destination_pos =
                    generator_pos.square_offset(&transport.direction.into(), map_size);
                let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));

                if let Some(destination_entity) = destination_entity {
                    transport.mu += mu_speed;
                    if transport.mu > 1.0 {
                        transport.mu = 0.0;
                        commands
                            .entity(payload_entity)
                            .insert(PayloadOf(destination_entity));
                    }
                } else {
                    // destination has gone - for generators this means we'll
                    // move it back to the origin
                    transport.mu -= mu_speed;
                    if transport.mu < 0.0 {
                        commands.entity(payload_entity).remove::<PayloadTransport>();
                    }
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_generator_payloads(
    generators: Query<(Entity, &TilePos, &Payloads), With<Generator>>,
    mut payloads: Query<(&PayloadOf, Option<&PayloadTransport>, &mut Transform)>,
    base: Single<
        (
            &TileStorage,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<BaseLayer>,
    >,
) {
    let (storage, map_size, grid_size, tile_size, map_type, anchor) = base.into_inner();

    for (generator, tile_pos, generator_payloads) in generators {
        let tile_center =
            tile_pos.center_in_world(map_size, grid_size, tile_size, map_type, anchor);

        for payload_entity in generator_payloads.iter() {
            let (payload, transport, mut transform) = payloads.get_mut(payload_entity).unwrap();

            let pos = if let Some(transport) = transport {
                let half_size = Vec2::new(tile_size.x / 2.0, tile_size.y / 2.0);

                let offset = match transport.direction {
                    ConveyorDirection::North => Vec2::new(0.0, half_size.y),
                    ConveyorDirection::South => Vec2::new(0.0, -half_size.y),
                    ConveyorDirection::East => Vec2::new(half_size.x, 0.0),
                    ConveyorDirection::West => Vec2::new(-half_size.x, 0.0),
                };

                tile_center.lerp(tile_center + offset, transport.mu)
            } else {
                tile_center
            };
            *transform = Transform::from_translation(pos.extend(2.0));
        }
    }
}
