use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        helpers::{CONVEYOR_DIRECTIONS, ConveyorDirection, get_neighbors_from_query, opposite},
        payload::{OfferPayloadEvent, PayloadOf, Payloads, TookPayloadEvent},
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
                    update_generator_payload_transports,
                )
                    .chain()
                    .in_set(ConveyorSystems::TransportLogic),
                update_generator_payloads.in_set(ConveyorSystems::PayloadTransforms),
            ),
        )
        .add_systems(PostUpdate, cleanup_generated_payload_transports);
    }
}

#[derive(Component, Default, Debug)]
pub struct Generator {
    next_generate_time: f32,
    next_transport_direction_index: u32,
}

#[derive(Component, Default, Debug, Reflect)]
struct GeneratedPayloadTransport {
    mu: f32,
    direction: ConveyorDirection,
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
                StateScoped(GameState::FactoryGame),
                Name::new("Payload"),
                PayloadOf(entity),
                Text2d::new("X"),
                TextColor(Color::linear_rgb(1.0, 0.4, 0.4)),
            ));

            generator.next_generate_time = time.elapsed_secs() + 5.0;
        }
    }
}

fn set_generator_payload_transports(
    mut commands: Commands,
    untransported_payloads: Query<(Entity, &PayloadOf), Without<GeneratedPayloadTransport>>,
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
                let direction = CONVEYOR_DIRECTIONS[direction_index];

                if conveyor_neighbors.get(direction.into()).is_some() {
                    commands
                        .entity(untransported_entity)
                        .insert(GeneratedPayloadTransport { mu: 0.0, direction });
                    generator.next_transport_direction_index =
                        ((direction_index + 1) % CONVEYOR_DIRECTIONS.len()) as u32;
                    break;
                }
            }
        }
    }
}

fn update_generator_payload_transports(
    mut commands: Commands,
    time: Res<Time>,
    generated_payloads: Query<(Entity, &mut GeneratedPayloadTransport, &PayloadOf)>,
    generators: Query<&TilePos, With<Generator>>,
    conveyors: Query<&Conveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut offer_payload_event: EventWriter<OfferPayloadEvent>,
) {
    let mu_speed = time.delta_secs();

    let (tile_storage, map_size) = base.into_inner();

    for (payload_entity, mut generated_payload, payload_of) in generated_payloads {
        let generator_entity = payload_of.0;
        if let Ok(generator_pos) = generators.get(generator_entity) {
            let destination_pos =
                generator_pos.square_offset(&generated_payload.direction.into(), map_size);
            let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));
            let destination_conveyor =
                destination_entity.and_then(|entity| conveyors.get(entity).ok());

            if let Some(destination_conveyor) = destination_conveyor {
                generated_payload.mu += mu_speed;
                if generated_payload.mu > 1.0 {
                    generated_payload.mu = 1.0;
                    offer_payload_event.write(OfferPayloadEvent {
                        source_direction: opposite(generated_payload.direction.into()).into(),
                        payload: payload_entity,
                        target: destination_entity.unwrap(),
                    });
                }
            } else {
                // destination has gone - for generators this means we'll
                // move it back to the origin
                generated_payload.mu -= mu_speed;
                if generated_payload.mu < 0.0 {
                    commands
                        .entity(payload_entity)
                        .remove::<GeneratedPayloadTransport>();
                }
            }
        }
    }
}

fn cleanup_generated_payload_transports(
    mut commands: Commands,
    mut took_payload_events: EventReader<TookPayloadEvent>,
) {
    for took_payload_event in took_payload_events.read() {
        commands
            .entity(took_payload_event.payload)
            .remove::<GeneratedPayloadTransport>();
    }
}

#[allow(clippy::type_complexity)]
fn update_generator_payloads(
    generators: Query<(&TilePos, &Payloads), With<Generator>>,
    mut payloads: Query<(Option<&GeneratedPayloadTransport>, &mut Transform)>,
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

    for (tile_pos, generator_payloads) in generators {
        let tile_center =
            tile_pos.center_in_world(map_size, grid_size, tile_size, map_type, anchor);

        for payload_entity in generator_payloads.iter() {
            let (transport, mut transform) = payloads.get_mut(payload_entity).unwrap();

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
