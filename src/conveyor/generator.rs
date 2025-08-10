use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::conveyor::{ConveyorSystems, visuals::BaseLayer};

pub struct GeneratorPlugin;
impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_generator_tiles.in_set(ConveyorSystems::Updater));
    }
}

#[derive(Component)]
pub struct Generator;

fn update_generator_tiles(
    mut commands: Commands,
    new_generators: Query<Entity, (With<Generator>, Without<TileTextureIndex>)>,
    mut removed_generators: RemovedComponents<Generator>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TileStorage>)>,
) {
    for new_generator in new_generators {
        commands.entity(new_generator).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: TileTextureIndex(30),
            ..default()
        });
    }

    for removed_generator in removed_generators.read() {
        commands
            .entity(removed_generator)
            .remove::<TileTextureIndex>();
    }
}
