use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{AcceptsPayloadConveyor, BridgeConveyor, Conveyor},
        helpers::ConveyorDirections,
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
    },
    sprite_sheet::GameSprite,
};

pub struct BridgePlugin;
impl Plugin for BridgePlugin {
    fn build(&self, app: &mut App) {
        app.register_place_tile_event::<PlaceBridgeEvent>()
            .add_systems(
                Update,
                update_bridge_tiles.in_set(ConveyorSystems::TileUpdater),
            );
    }
}

pub struct BridgeTool;
impl Tool for BridgeTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Bridge, TileFlip::default())
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

#[derive(Component)]
pub struct Bridge;

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
            bridge_conveyor: BridgeConveyor,
            bridge: Bridge,
            accepts_payload: AcceptsPayloadConveyor::default(),
        }
    }
}

fn update_bridge_tiles(
    mut commands: Commands,
    new_bridges: Query<Entity, Added<Bridge>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TilemapSize>)>,
) {
    for e in new_bridges {
        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: GameSprite::Bridge.tile_texture_index(),
            ..default()
        });
    }
}
