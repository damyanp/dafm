use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        conveyor::Conveyor, helpers::ConveyorDirections, interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool}, payloads::{PayloadTransferredEvent, RequestPayloadTransferEvent}, BaseLayer, ConveyorSystems
    },
    sprite_sheet::GameSprite,
};

pub fn sink_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceSinkEvent>()
        .add_systems(
            Update,
            (
                update_sink_tiles.in_set(ConveyorSystems::TileUpdater),
                transfer_payloads_to_sinks.in_set(ConveyorSystems::TransferPayloads),
            ),
        );
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
        commands.insert((Sink, Name::new("Sink")));
    }
}

#[derive(Component)]
#[require(Conveyor::new(ConveyorDirections::default()))]
struct Sink;

fn transfer_payloads_to_sinks(
    mut commands: Commands,
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    sinks: Query<&Sink>,
    mut transferred: EventWriter<PayloadTransferredEvent>,
) {
    for e in transfers.read() {
        if sinks.contains(e.destination) {
            transferred.write(PayloadTransferredEvent {
                payload: e.payload,
                source: e.source,
            });
            commands.entity(e.payload).despawn();
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
