use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use smallvec::SmallVec;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        helpers::{ConveyorDirection, ConveyorDirections},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payload_handler::{AddPayloadHandler, PayloadHandler},
        payloads::{Payload, RequestPayloadTransferEvent, get_payload_transform},
    },
    helpers::TilemapQuery,
    sprite_sheet::GameSprite,
};

pub fn sink_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceSinkEvent>()
        .add_payload_handler::<Sink>()
        .add_systems(
            Update,
            (
                update_sink_tiles.in_set(ConveyorSystems::TileUpdater),
                update_sinks.in_set(ConveyorSystems::TransportLogic),
                update_sink_transforms.in_set(ConveyorSystems::PayloadTransforms),
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
        commands.insert((Sink::default(), Name::new("Sink")));
    }
}

#[derive(Component, Reflect, Default)]
#[require(Conveyor::new(ConveyorDirections::default()))]
struct Sink {
    payloads: SmallVec<[(Entity, ConveyorDirection, f32); 4]>,
}

impl PayloadHandler for Sink {
    fn try_transfer(
        &mut self,
        _: &Conveyor,
        request: &RequestPayloadTransferEvent,
    ) -> Option<Entity> {
        self.payloads
            .push((request.payload, request.direction.opposite(), 0.0));
        Some(request.payload)
    }

    fn remove_payload(&mut self, _: Entity) {
        panic!("Sink should never transfer a payload to another handler!");
    }

    fn iter_payloads(&self) -> impl Iterator<Item = Entity> {
        self.payloads.iter().map(|(e, _, _)| *e)
    }
}

fn update_sinks(mut commands: Commands, time: Res<Time>, sinks: Query<&mut Sink>) {
    let t = time.delta_secs();

    for mut sink in sinks {
        for (entity, _, mu) in &mut sink.payloads {
            *mu += t;
            if *mu >= 1.0 {
                commands.entity(*entity).despawn();
            }
        }
        sink.payloads.retain(|(_, _, mu)| *mu < 1.0);
    }
}

fn update_sink_transforms(
    sinks: Query<(&TilePos, &Sink)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, sink) in sinks {
        let tile_center = base.center_in_world(tile_pos);
        for (entity, direction, mu) in &sink.payloads {
            if let Ok(mut transform) = payloads.get_mut(*entity) {
                let payload_transform =
                    get_payload_transform(tile_center, base.tile_size, Some(*direction), None, *mu);

                let scale_mu = 1.0 - ((*mu - 0.5) * 2.0).max(0.0);

                *transform = payload_transform * Transform::from_scale(Vec3::splat(scale_mu));
            }
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
