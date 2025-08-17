use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{AcceptsPayloadConveyor, Conveyor, Payloads},
        helpers::ConveyorDirections,
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
    },
    sprite_sheet::GameSprite,
};

pub struct SinkPlugin;
impl Plugin for SinkPlugin {
    fn build(&self, app: &mut App) {
        app.register_place_tile_event::<PlaceSinkEvent>()
            .add_systems(
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
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Sink, TileFlip::default())
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceSinkEvent(*tile_pos));
    }
}

#[derive(Event, Debug)]
pub struct PlaceSinkEvent(TilePos);

impl PlaceTileEvent for PlaceSinkEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
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
            conveyor: Conveyor::new(ConveyorDirections::default()),
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
            texture_index: GameSprite::Sink.tile_texture_index(),
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
