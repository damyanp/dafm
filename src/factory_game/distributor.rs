use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::factory_game::{
    BaseLayer, ConveyorSystems,
    conveyor::{AcceptsPayloadConveyor, Conveyor, DistributorConveyor},
    helpers::ConveyorDirections,
};

pub struct DistributorPlugin;
impl Plugin for DistributorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_distributor_tiles,).in_set(ConveyorSystems::TileUpdater),
        );
    }
}

#[derive(Component, PartialEq, Eq, Hash)]
struct Distributor;

#[derive(Bundle)]
pub struct DistributorBundle {
    conveyor: Conveyor,
    distributor: Distributor,
    distributor_conveyor: DistributorConveyor,
    accepts_payload: AcceptsPayloadConveyor,
}

impl DistributorBundle {
    pub fn new() -> Self {
        DistributorBundle {
            conveyor: Conveyor {
                outputs: ConveyorDirections::all(),
                accepts_input: true,
            },
            distributor: Distributor,
            distributor_conveyor: DistributorConveyor::default(),
            accepts_payload: AcceptsPayloadConveyor,
        }
    }
}

fn update_distributor_tiles(
    mut commands: Commands,
    new_distributors: Query<Entity, Added<Distributor>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TilemapSize>)>,
) {
    for e in new_distributors {
        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: TileTextureIndex(32),
            ..default()
        });
    }
}
