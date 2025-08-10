use std::ops::DerefMut;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::SquareDirection, prelude::*};
use bevy_egui::input::{egui_wants_any_keyboard_input, egui_wants_any_pointer_input};

use super::{Conveyor, MapConfig, helpers::*, visuals::BaseLayer};
use crate::GameState;

pub struct InteractionPlugin;
impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HoveredTile>()
            .add_systems(OnEnter(GameState::Conveyor), startup)
            .add_systems(Update, track_mouse)
            .add_systems(
                Update,
                (
                    (
                        on_click.run_if(input_just_pressed(MouseButton::Left)),
                        on_space.run_if(input_just_pressed(KeyCode::Space)),
                    )
                        .run_if(not(egui_wants_any_keyboard_input))
                        .run_if(not(egui_wants_any_pointer_input)),
                    update_hovered_tile,
                )
                    .chain()
                    .run_if(in_state(GameState::Conveyor)),
            );
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<MapConfig>) {
    let texture = asset_server.load("sprites.png");
    let interaction_layer = commands
        .spawn(make_interaction_layer(&config, texture.to_owned()))
        .id();

    commands.spawn((
        StateScoped(GameState::Conveyor),
        Name::new("HoveredTile"),
        HoveredTile(None),
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
) {
    let (tile_pos, hovered_tile) = *hovered_tile;
    if let Some(e) = storage.get(tile_pos) {
        storage.remove(tile_pos);
        commands.entity(e).despawn();
    }
    if hovered_tile.0.is_some() {
        storage.set(
            tile_pos,
            commands
                .spawn((
                    StateScoped(GameState::Conveyor),
                    Name::new("Placed Tile"),
                    Conveyor(hovered_tile.0.unwrap()),
                    *tile_pos,
                ))
                .id(),
        );
    }
}

fn on_space(mut hovered_tile: Single<&mut HoveredTile>) {
    hovered_tile.set_to_next_option();
}

fn update_hovered_tile(
    mut q: Single<(Entity, &HoveredTile, &mut TileTextureIndex, &mut TileFlip)>,
) {
    if let HoveredTile(Some(hovered_direction)) = q.1 {
        let (_, _, texture_index, flip) = q.deref_mut();
        (**texture_index, **flip) = get_hover_tile((*hovered_direction).into());
    } else {
        *q.2 = TileTextureIndex(20);
    }
}

#[derive(Component, Reflect)]
struct HoveredTile(Option<ConveyorDirection>);

impl HoveredTile {
    fn set_to_next_option(&mut self) {
        use ConveyorDirection::*;

        self.0 = match self.0 {
            None => Some(East),
            Some(East) => Some(South),
            Some(South) => Some(West),
            Some(West) => Some(North),
            Some(North) => None,
        };
    }
}

const DIRECTION_ARROW: TileTextureIndex = TileTextureIndex(22);

fn get_hover_tile(direction: SquareDirection) -> (TileTextureIndex, TileFlip) {
    use SquareDirection::*;

    let flip = match direction {
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
        _ => panic!(),
    };

    (DIRECTION_ARROW, flip)

    // get_conveyor_tile(opposite(direction), direction)
}

#[derive(Component)]
pub struct InteractionLayer;

fn make_interaction_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (
        InteractionLayer,
        super::make_layer(config, texture, 1.0, "InteractionLayer"),
    )
}
