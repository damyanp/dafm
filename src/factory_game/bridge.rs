use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{Conveyor, TilesToCheck},
        helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query, opposite},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payloads::{PayloadTransferredEvent, PayloadTransportLine, RequestPayloadTransferEvent},
    },
    helpers::TilemapQuery,
    sprite_sheet::{GameSprite, SpriteSheet},
};

pub fn bridge_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceBridgeEvent>()
        .register_type::<BridgeConveyor>()
        .add_systems(
            Update,
            (
                (update_bridge_conveyor_accepts_payload, update_bridge_tiles)
                    .in_set(ConveyorSystems::TileUpdater),
                transfer_bridge_payloads.in_set(ConveyorSystems::TransferPayloads),
                update_bridge_payloads.in_set(ConveyorSystems::TransportLogic),
            ),
        );
}

pub struct BridgeTool;
impl Tool for BridgeTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::BridgeBoth, TileFlip::default())
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceBridgeEvent(*tile_pos));
    }
}

#[derive(Event, Debug)]
pub struct PlaceBridgeEvent(pub TilePos);

impl PlaceTileEvent for PlaceBridgeEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((Bridge::default(), Name::new("Bridge")));
    }
}

#[derive(Component, Default, Reflect, Debug)]
pub struct BridgeConveyor {
    top: Option<PayloadTransportLine>,
    bottom: Option<PayloadTransportLine>,
}

#[derive(Component, Default)]
#[relationship_target(relationship = BridgeTop, linked_spawn)]
#[require(Conveyor::new(ConveyorDirections::all()), BridgeConveyor)]
pub struct Bridge(Vec<Entity>);

/// Mark BridgeTops so they can be despawned when the Bridge is despawned
#[derive(Component)]
#[relationship(relationship_target = Bridge)]
pub struct BridgeTop(Entity);

/// Bridge conveyors need to look at which neighbors are set to output to this
/// one to figure out where their inputs are.
fn update_bridge_conveyor_accepts_payload(
    to_check: Res<TilesToCheck>,
    mut bridge_conveyors: Query<&mut BridgeConveyor>,
    conveyors: Query<&Conveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for pos in &to_check.0 {
        if let Some(entity) = tile_storage.get(pos)
            && let Ok(bridge) = bridge_conveyors.get_mut(entity)
        {
            let neighbor_conveyors =
                get_neighbors_from_query(tile_storage, pos, map_size, &conveyors);

            let inputs =
                neighbor_conveyors
                    .iter_with_direction()
                    .filter_map(|(direction, conveyor)| {
                        if conveyor.outputs().is_set(opposite(direction).into()) {
                            Some(ConveyorDirection::from(direction))
                        } else {
                            None
                        }
                    });
        }
    }
}

fn update_bridge_payloads() {}

fn transfer_bridge_payloads(
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    mut bridges: Query<&mut BridgeConveyor>,
    mut transferred: EventWriter<PayloadTransferredEvent>,
) {
    for RequestPayloadTransferEvent {
        payload,
        source,
        destination,
        direction,
    } in transfers.read()
    {
        if let Ok(mut bridge) = bridges.get_mut(*destination) {
            use ConveyorDirection::*;

            let transport = match direction {
                North | South => bridge.bottom.as_mut(),
                East | West => bridge.top.as_mut(),
            };

            let take = transport
                .map(|transport| transport.try_transfer_onto(*payload, direction.opposite()))
                .unwrap_or(false);

            if take {
                transferred.write(PayloadTransferredEvent {
                    payload: *payload,
                    source: *source,
                });
            }
        }
    }
}

fn update_bridge_tiles(
    mut commands: Commands,
    new_bridges: Query<(Entity, &TilePos), Added<Bridge>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
    sprite_sheet: Res<SpriteSheet>,
) {
    for (e, tile_pos) in new_bridges {
        let tile_center = base.center_in_world(tile_pos);

        commands.spawn((
            Name::new("Bridge Top"),
            sprite_sheet.sprite(GameSprite::BridgeTop),
            Transform::from_translation(tile_center.extend(2.0)),
            BridgeTop(e),
        ));

        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(base.entity),
            texture_index: GameSprite::BridgeBottom.tile_texture_index(),
            ..default()
        });
    }
}
