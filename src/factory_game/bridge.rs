use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{Conveyor, TilesToCheck},
        conveyor_belts::find_incoming_directions,
        helpers::{ConveyorDirection, ConveyorDirections},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payload_handler::{AddPayloadHandler, PayloadHandler},
        payloads::{Payload, PayloadTransportLine, RequestPayloadTransferEvent},
    },
    helpers::{TilemapQuery, TilemapQueryItem},
    sprite_sheet::{GameSprite, SpriteSheet},
};

pub fn bridge_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceBridgeEvent>()
        .add_payload_handler::<BridgeConveyor>()
        .add_systems(
            Update,
            (
                (update_bridge_conveyors, update_bridge_tiles).in_set(ConveyorSystems::TileUpdater),
                update_bridge_payloads.in_set(ConveyorSystems::TransportLogic),
                update_bridge_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
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

#[derive(Component, Reflect, Debug)]
pub struct BridgeConveyor {
    top: Option<PayloadTransportLine>,
    bottom: Option<PayloadTransportLine>,
    capacity: u32,
}

impl Default for BridgeConveyor {
    fn default() -> Self {
        Self {
            top: Default::default(),
            bottom: Default::default(),
            capacity: 5,
        }
    }
}

impl PayloadHandler for BridgeConveyor {
    fn try_transfer(
        &mut self,
        _: &Conveyor,
        request: &RequestPayloadTransferEvent,
    ) -> Option<Entity> {
        use ConveyorDirection::*;

        let transport = match request.direction {
            North | South => self.bottom.as_mut(),
            East | West => self.top.as_mut(),
        };

        transport.and_then(|transport| {
            transport.try_transfer_onto(request.direction.opposite(), || request.payload)
        })
    }

    fn remove_payload(&mut self, payload: Entity) {
        if let Some(top) = &mut self.top {
            top.remove_payload(payload);
        }
        if let Some(bottom) = &mut self.bottom {
            bottom.remove_payload(payload);
        }
    }

    fn iter_payloads(&self) -> impl Iterator<Item = Entity> {
        std::iter::empty()
            .chain(self.top.iter().flat_map(|t| t.iter_payloads()))
            .chain(self.bottom.iter().flat_map(|b| b.iter_payloads()))
    }
}

impl BridgeConveyor {
    fn update_payload_transforms(
        &self,
        tile_pos: &TilePos,
        payloads: &mut Query<&mut Transform, With<Payload>>,
        base: &TilemapQueryItem,
    ) {
        if let Some(top) = &self.top {
            top.update_payload_transforms(tile_pos, payloads, base);
        }
        if let Some(bottom) = &self.bottom {
            bottom.update_payload_transforms(tile_pos, payloads, base);
        }
    }

    fn current_bottom_output(&self) -> Option<ConveyorDirection> {
        self.bottom.as_ref().map(|bottom| bottom.output_direction())
    }

    fn current_top_output(&self) -> Option<ConveyorDirection> {
        self.top.as_ref().map(|top| top.output_direction())
    }
}

#[derive(Component, Default)]
#[relationship_target(relationship = BridgeTop, linked_spawn)]
#[require(Conveyor::new(ConveyorDirections::default()), BridgeConveyor)]
pub struct Bridge(Vec<Entity>);

/// Mark BridgeTops so they can be despawned when the Bridge is despawned
#[derive(Component)]
#[relationship(relationship_target = Bridge)]
pub struct BridgeTop(Entity);

/// Bridge conveyors need to look at which neighbors are set to output to this
/// one to figure out where their inputs are.
fn update_bridge_conveyors(
    to_check: Res<TilesToCheck>,
    mut bridge_conveyors: Query<&mut BridgeConveyor>,
    mut conveyors: Query<&mut Conveyor>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for tile_pos in &to_check.0 {
        if let Some(entity) = tile_storage.get(tile_pos)
            && let Ok(mut bridge) = bridge_conveyors.get_mut(entity)
        {
            let inputs = find_incoming_directions(
                tile_pos,
                tile_storage,
                map_size,
                &conveyors.as_readonly(),
            );

            if let Ok(mut conveyor) = conveyors.get_mut(entity) {
                conveyor.set_inputs(inputs);

                use ConveyorDirection::*;

                let wanted_bottom_output = if inputs.is_set(North) {
                    Some(South)
                } else if inputs.is_set(South) {
                    Some(North)
                } else {
                    None
                };

                let wanted_top_output = if inputs.is_set(East) {
                    Some(West)
                } else if inputs.is_set(West) {
                    Some(East)
                } else {
                    None
                };

                let mut outputs = ConveyorDirections::default();
                wanted_bottom_output
                    .iter()
                    .chain(wanted_bottom_output.iter())
                    .for_each(|output| outputs.add(*output));

                conveyor.set_outputs(outputs);

                let current_bottom_output = bridge.current_bottom_output();
                if current_bottom_output != wanted_bottom_output {
                    bridge.bottom = wanted_bottom_output
                        .map(|output| PayloadTransportLine::new(output, bridge.capacity));
                }
                let current_top_output = bridge.current_top_output();
                if current_top_output != wanted_top_output {
                    bridge.top = wanted_top_output
                        .map(|output| PayloadTransportLine::new(output, bridge.capacity));
                }
            }
        }
    }
}

fn update_bridge_payloads(
    bridges: Query<(Entity, &mut BridgeConveyor, &TilePos)>,
    time: Res<Time>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();

    let t = time.delta_secs();

    for (source, mut bridge, tile_pos) in bridges {
        if let Some(top) = &mut bridge.top {
            top.update(
                source,
                tile_pos,
                t,
                tile_storage,
                map_size,
                &mut send_payloads,
            );
        }
        if let Some(bottom) = &mut bridge.bottom {
            bottom.update(
                source,
                tile_pos,
                t,
                tile_storage,
                map_size,
                &mut send_payloads,
            );
        }
    }
}

fn update_bridge_payload_transforms(
    bridges: Query<(&TilePos, &BridgeConveyor)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, bridge) in bridges {
        bridge.update_payload_transforms(tile_pos, &mut payloads, &base);
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
