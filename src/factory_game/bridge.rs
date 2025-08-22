use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, BaseLayerEntityDespawned, ConveyorSystems,
        conveyor::{AcceptsPayloadConveyor, Conveyor, find_tiles_to_check},
        helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query, opposite},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payloads::{Payload, PayloadTransport, RequestPayloadTransferEvent},
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
        commands.insert((BridgeBundle::new(), Name::new("Bridge")));
    }
}

#[derive(Component, Default, Reflect, Debug)]
pub struct BridgeConveyor {
    top: Vec<Entity>,
    bottom: Vec<Entity>,
}

#[derive(Component, Default)]
#[relationship_target(relationship = BridgeTop, linked_spawn)]
pub struct Bridge(Vec<Entity>);

/// Mark BridgeTops so they can be despawned when the Bridge is despawned
#[derive(Component)]
#[relationship(relationship_target = Bridge)]
pub struct BridgeTop(Entity);

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
            conveyor: Conveyor::new(ConveyorDirections::all()),
            bridge_conveyor: BridgeConveyor::default(),
            bridge: Bridge::default(),
            accepts_payload: AcceptsPayloadConveyor::default(),
        }
    }
}

/// Bridge conveyors need to look at which neighbors are set to output to this
/// one to figure out where their inputs are.
fn update_bridge_conveyor_accepts_payload(
    new_conveyors: Query<&TilePos, Added<Conveyor>>,
    removed_entities: EventReader<BaseLayerEntityDespawned>,
    mut accepts_payloads: Query<&mut AcceptsPayloadConveyor, With<BridgeConveyor>>,
    conveyors: Query<&Conveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    let to_check = find_tiles_to_check(new_conveyors, removed_entities, map_size);

    for pos in to_check {
        if let Some(entity) = tile_storage.get(&pos)
            && let Ok(mut accepts) = accepts_payloads.get_mut(entity)
        {
            let neighbor_conveyors =
                get_neighbors_from_query(tile_storage, &pos, map_size, &conveyors);

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

            *accepts = AcceptsPayloadConveyor::new(ConveyorDirections::from(inputs));
        }
    }
}

fn update_bridge_payloads(
    bridges: Query<(Entity, &mut BridgeConveyor)>,
    payloads: Query<&Payload>,
) {
    for (bridge_entity, mut bridge) in bridges {
        // TODO: This polls every bridge....which isn't great...but probably not
        // really a problem right now.

        let is_on_bridge = |entity: &Entity| {
            payloads
                .get(*entity)
                .map(|payload_of| payload_of.0 == bridge_entity)
                .unwrap_or(false)
        };

        bridge.top.retain(is_on_bridge);
        bridge.bottom.retain(is_on_bridge);
    }
}

fn transfer_bridge_payloads(
    mut commands: Commands,
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    mut bridges: Query<&mut BridgeConveyor>,
) {
    for RequestPayloadTransferEvent {
        payload,
        destination,
        direction,
    } in transfers.read()
    {
        if let Ok(mut bridge) = bridges.get_mut(*destination) {
            use ConveyorDirection::*;
            let take = match direction {
                North | South => {
                    if bridge.bottom.is_empty() {
                        bridge.bottom.push(*payload);
                        true
                    } else {
                        false
                    }
                }
                East | West => {
                    if bridge.top.is_empty() {
                        bridge.top.push(*payload);
                        true
                    } else {
                        false
                    }
                }
            };

            if take {
                commands.entity(*payload).insert((
                    Payload(*destination),
                    PayloadTransport {
                        source: Some(direction.opposite()),
                        destination: Some(*direction),
                        ..default()
                    },
                ));
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
