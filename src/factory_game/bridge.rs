use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::factory_game::{
    BaseLayer, ConveyorSystems,
    conveyor::{AcceptsPayloadConveyor, BridgeConveyor, Conveyor},
    helpers::ConveyorDirections,
};

pub struct BridgePlugin;
impl Plugin for BridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_bridge_tiles.in_set(ConveyorSystems::TileUpdater),
        );
    }
}

#[derive(Component)]
struct Bridge;

#[derive(Bundle)]
pub struct BridgeBundle {
    conveyor: Conveyor,
    bridge_conveyor: BridgeConveyor,
    bridge: Bridge,
    accepts_payload: AcceptsPayloadConveyor,
}

impl BridgeBundle {
    pub fn new() -> Self {
        BridgeBundle {
            conveyor: Conveyor {
                outputs: ConveyorDirections::all(),
                accepts_input: true,
            },
            bridge_conveyor: BridgeConveyor,
            bridge: Bridge,
            accepts_payload: AcceptsPayloadConveyor,
        }
    }
}

fn update_bridge_tiles(
    mut commands: Commands,
    new_bridges: Query<Entity, Added<Bridge>>,
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
