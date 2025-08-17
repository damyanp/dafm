use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{AcceptsPayloadConveyor, Conveyor, DistributorConveyor},
        helpers::ConveyorDirections,
        interaction::Tool,
    },
    sprite_sheet::GameSprite,
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

pub struct DistributorTool;
impl Tool for DistributorTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Distributor, TileFlip::default())
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((DistributorBundle::new(), Name::new("Distributor")));
    }
}

#[derive(Component)]
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
            conveyor: Conveyor::new(ConveyorDirections::all()),
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
            texture_index: GameSprite::Distributor.tile_texture_index(),
            ..default()
        });
    }
}
