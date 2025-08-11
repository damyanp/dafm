use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::input::{egui_wants_any_keyboard_input, egui_wants_any_pointer_input};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, BaseLayerEntityDespawned, ConveyorSystems, MapConfig, conveyor::Conveyor,
        conveyor_belts::ConveyorBelt, generator::Generator, helpers::*,
    },
};

pub struct ConveyorInteractionPlugin;
impl Plugin for ConveyorInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HoveredTile>()
            .add_systems(OnEnter(GameState::FactoryGame), startup)
            .add_systems(
                Update,
                (
                    (
                        (
                            track_mouse,
                            on_click.run_if(input_just_pressed(MouseButton::Left)),
                        )
                            .chain()
                            .run_if(not(egui_wants_any_pointer_input)),
                        on_space
                            .run_if(input_just_pressed(KeyCode::Space))
                            .run_if(not(egui_wants_any_keyboard_input)),
                    )
                        .in_set(ConveyorSystems::TileGenerator),
                    update_hovered_tile.in_set(ConveyorSystems::TileUpdater),
                ),
            );
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<MapConfig>) {
    let texture = asset_server.load("sprites.png");
    let interaction_layer = commands
        .spawn(make_interaction_layer(&config, texture.to_owned()))
        .id();

    commands.spawn((
        StateScoped(GameState::FactoryGame),
        Name::new("HoveredTile"),
        HoveredTile::None,
        TileBundle {
            texture_index: TileTextureIndex(20),
            tilemap_id: TilemapId(interaction_layer),
            ..default()
        },
    ));
}

#[allow(clippy::type_complexity)]
fn track_mouse(
    mut cursor_moved: EventReader<CursorMoved>,
    camera_query: Single<(&GlobalTransform, &Camera)>,
    interaction_layer: Single<
        (
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<InteractionLayer>,
    >,
    mut hovered_tile: Single<&mut TilePos, With<HoveredTile>>,
) {
    if let Some(e) = cursor_moved.read().last() {
        let (global_transform, camera) = *camera_query;
        if let Ok(p) = camera.viewport_to_world_2d(global_transform, e.position) {
            let (size, grid_size, tile_size, map_type, anchor) = *interaction_layer;

            if let Some(tile_pos) =
                TilePos::from_world_pos(&p, size, grid_size, tile_size, map_type, anchor)
            {
                **hovered_tile = tile_pos;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn on_click(
    mut commands: Commands,
    hovered_tile: Single<(&TilePos, &HoveredTile)>,
    mut storage: Single<&mut TileStorage, With<BaseLayer>>,
    mut despawned_event: EventWriter<BaseLayerEntityDespawned>,
) {
    let (tile_pos, hovered_tile) = *hovered_tile;

    if let Some(old_entity) = storage.remove(tile_pos) {
        commands.entity(old_entity).despawn();
        despawned_event.write(BaseLayerEntityDespawned(*tile_pos));
    }

    if *hovered_tile != HoveredTile::None {
        let entity = commands
            .spawn((
                StateScoped(GameState::FactoryGame),
                Name::new("Placed Tile"),
                BaseLayer,
                *tile_pos,
            ))
            .id();
        storage.set(tile_pos, entity);

        match hovered_tile {
            HoveredTile::Conveyor(direction) => {
                commands.entity(entity).insert((
                    Conveyor {
                        outputs: ConveyorDirections::new(*direction),
                        accepts_input: true,
                    },
                    ConveyorBelt,
                ));
            }
            HoveredTile::Source => {
                commands.entity(entity).insert(Generator::default());
            }
            HoveredTile::None => panic!(),
        }
    }
}

fn on_space(mut hovered_tile: Single<&mut HoveredTile>) {
    hovered_tile.set_to_next_option();
}

fn update_hovered_tile(q: Single<(&HoveredTile, &mut TileTextureIndex, &mut TileFlip)>) {
    let (hovered_tile, mut texture_index, mut flip) = q.into_inner();
    (*texture_index, *flip) = get_hovered_tile_texture(hovered_tile);
}

#[derive(Component, Reflect, PartialEq, Eq)]
enum HoveredTile {
    None,
    Conveyor(ConveyorDirection),
    Source,
}

impl HoveredTile {
    fn set_to_next_option(&mut self) {
        use ConveyorDirection::*;
        use HoveredTile::*;

        *self = match self {
            None => Conveyor(East),
            Conveyor(East) => Conveyor(South),
            Conveyor(South) => Conveyor(West),
            Conveyor(West) => Conveyor(North),
            Conveyor(North) => Source,
            Source => None,
        };
    }
}

const DIRECTION_ARROW: TileTextureIndex = TileTextureIndex(22);

fn get_hovered_tile_texture(hovered_tile: &HoveredTile) -> (TileTextureIndex, TileFlip) {
    use ConveyorDirection::*;
    use HoveredTile::*;

    match hovered_tile {
        Conveyor(direction) => (
            DIRECTION_ARROW,
            match direction {
                East => TileFlip::default(),
                North => TileFlip {
                    d: true,
                    y: true,
                    ..default()
                },
                West => TileFlip {
                    x: true,
                    ..default()
                },
                South => TileFlip {
                    d: true,
                    ..default()
                },
            },
        ),
        Source => (TileTextureIndex(30), TileFlip::default()),
        None => (TileTextureIndex(20), TileFlip::default()),
    }
}

#[derive(Component)]
pub struct InteractionLayer;

fn make_interaction_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (
        InteractionLayer,
        super::make_layer(config, texture, 1.0, "InteractionLayer"),
    )
}
