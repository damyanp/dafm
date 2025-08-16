use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::factory_game::{
    BaseLayer, ConveyorSystems,
    conveyor::{AcceptsPayloadConveyor, Conveyor, Payloads},
    helpers::ConveyorDirections,
    interaction::Tool,
};

pub struct SinkPlugin;
impl Plugin for SinkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_sink_tiles.in_set(ConveyorSystems::TileUpdater),
                sink_despawns_everything_in_it.in_set(ConveyorSystems::TransportLogic),
            ),
        );
    }
}

pub struct SinkTool;

impl Tool for SinkTool {
    fn get_texture_index_flip(&self) -> (TileTextureIndex, TileFlip) {
        (TileTextureIndex(31), TileFlip::default())
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((SinkBundle::new(), Name::new("Sink")));
    }
}

#[derive(Component)]
struct Sink;

#[derive(Bundle)]
pub struct SinkBundle {
    sink: Sink,
    conveyor: Conveyor,
    accepts_payload: AcceptsPayloadConveyor,
}

impl SinkBundle {
    pub fn new() -> Self {
        SinkBundle {
            sink: Sink,
            conveyor: Conveyor {
                outputs: ConveyorDirections::default(),
                accepts_input: true,
            },
            accepts_payload: AcceptsPayloadConveyor,
        }
    }
}

fn update_sink_tiles(
    mut commands: Commands,
    new_sinks: Query<Entity, (With<Sink>, Without<TileTextureIndex>)>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TileStorage>)>,
) {
    for new_sink in new_sinks {
        commands.entity(new_sink).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: TileTextureIndex(31),
            ..default()
        });
    }
}

fn sink_despawns_everything_in_it(mut commands: Commands, sinks: Query<&Payloads, With<Sink>>) {
    for payloads in sinks {
        for entity in payloads.iter() {
            commands.entity(entity).despawn();
        }
    }
}
