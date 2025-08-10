use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    conveyor::{
        ConveyorSystems,
        helpers::ConveyorDirection,
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
                generate_payloads.in_set(ConveyorSystems::TransportLogic),
                update_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
            ),
        );
    }
}

#[derive(Component, Default, Debug)]
pub struct Generator {
    next_generate_time: f32,
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
                PayloadTransport {
                    mu: 0.0,
                    direction: ConveyorDirection::East,
                },
            ));

            generator.next_generate_time = time.elapsed_secs() + 60.0;
        }
    }
}

fn update_payload_transforms(
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
