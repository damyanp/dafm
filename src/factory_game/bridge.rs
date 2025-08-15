use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::factory_game::{
    BaseLayer, ConveyorSystems,
    conveyor::{
        AcceptsPayloadConveyor, Conveyor, PayloadDestination, PayloadSource, PayloadTransport,
        Payloads,
    },
    helpers::ConveyorDirections,
};

pub struct BridgePlugin;
impl Plugin for BridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_bridge_tiles.in_set(ConveyorSystems::TileUpdater),
                update_bridge_conveyor_destinations.in_set(ConveyorSystems::TransportLogic),
            ),
        );
    }
}

#[derive(Component)]
struct BridgeConveyor;

#[derive(Bundle)]
pub struct BridgeBundle {
    conveyor: Conveyor,
    bridge: BridgeConveyor,
    accepts_payload: AcceptsPayloadConveyor,
}

impl BridgeBundle {
    pub fn new() -> Self {
        BridgeBundle {
            conveyor: Conveyor {
                outputs: ConveyorDirections::all(),
                accepts_input: true,
            },
            bridge: BridgeConveyor,
            accepts_payload: AcceptsPayloadConveyor,
        }
    }
}

fn update_bridge_tiles(
    mut commands: Commands,
    new_bridges: Query<Entity, Added<BridgeConveyor>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TilemapSize>)>,
) {
    for e in new_bridges {
        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: TileTextureIndex(33),
            ..default()
        });
    }
}

fn update_bridge_conveyor_destinations(
    mut commands: Commands,
    bridge_conveyors: Query<&Payloads, With<BridgeConveyor>>,
    payloads: Query<(Entity, &PayloadSource), (With<PayloadTransport>, Without<PayloadDestination>)>,
) {
    for bridge_payloads in bridge_conveyors {

        for (payload, source) in payloads.iter_many(bridge_payloads.iter()) {
            commands
                .entity(payload)
                .insert(PayloadDestination(source.0.opposite()));
        }
    }
}
