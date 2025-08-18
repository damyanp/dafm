use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{AcceptsPayloadConveyor, BridgeConveyor, Conveyor},
        helpers::ConveyorDirections,
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
    },
    sprite_sheet::{GameSprite, SpriteSheet},
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

#[expect(clippy::type_complexity)]
fn update_bridge_tiles(
    mut commands: Commands,
    new_bridges: Query<(Entity, &TilePos), Added<Bridge>>,
    base: Single<
        (
            Entity,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<BaseLayer>,
    >,
    sprite_sheet: Res<SpriteSheet>,
) {
    let (tilemap_entity, map_size, grid_size, tile_size, map_type, anchor) = base.into_inner();

    for (e, tile_pos) in new_bridges {
        let tile_center =
            tile_pos.center_in_world(map_size, grid_size, tile_size, map_type, anchor);

        commands.spawn((
            Name::new("Bridge Top"),
            sprite_sheet.sprite(GameSprite::BridgeTop),
            Transform::from_translation(tile_center.extend(2.0)),
            BridgeTop(e),
        ));

        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(tilemap_entity),
            texture_index: GameSprite::BridgeBottom.tile_texture_index(),
            ..default()
        });
    }
}
