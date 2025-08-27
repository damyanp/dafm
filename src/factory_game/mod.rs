use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::PrimaryEguiContext;
use bevy_pancam::PanCam;

use crate::{GameState, helpers::set_camera_limits_from_tilemaps, sprite_sheet::SpriteSheet};

mod bridge;
mod conveyor;
mod conveyor_belts;
mod dev;
mod distributor;
mod generator;
mod helpers;
mod interaction;
mod operators;
mod payloads;
mod sink;
mod ui;

#[cfg(test)]
mod test;

use helpers::*;

pub fn factory_game_logic_plugin(app: &mut App) {
    app.add_plugins(bridge::bridge_plugin)
        .add_plugins(conveyor_belts::conveyor_belts_plugin)
        .add_plugins(conveyor::conveyor_plugin)
        .add_plugins(payloads::payloads_plugin)
        .add_plugins(distributor::distributor_plugin)
        .add_plugins(generator::generator_plugin)
        .add_plugins(operators::operators_plugin)
        .add_plugins(sink::sink_plugin)
        .insert_resource(MapConfig::default())
        .add_event::<BaseLayerEntityDespawned>()
        .configure_sets(
            Update,
            (
                ConveyorSystems::TileGenerator,
                ConveyorSystems::TileUpdater,
                ConveyorSystems::TransferPayloads,
                ConveyorSystems::TransferredPayloads,
                ConveyorSystems::TransportLogic,
                ConveyorSystems::PayloadTransforms,
            )
                .chain()
                .run_if(in_state(GameState::FactoryGame)),
        );
}

pub fn factory_game_plugin(app: &mut App) {
    app //
        .add_plugins(interaction::interaction_plugin)
        .add_plugins(factory_game_logic_plugin)
        .add_plugins(dev::dev_plugin)
        .add_plugins(ui::ui_plugin)
        .add_systems(
            OnEnter(GameState::FactoryGame),
            (
                make_base_layer,
                setup_camera,
                set_camera_limits_from_tilemaps,
            )
                .chain(),
        );
}

fn setup_camera(mut commands: Commands) {
    let pan_cam = PanCam {
        grab_buttons: vec![MouseButton::Middle],
        ..default()
    };

    commands.spawn((
        StateScoped(GameState::FactoryGame),
        Camera2d,
        PrimaryEguiContext,
        pan_cam,
    ));
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum ConveyorSystems {
    TileGenerator,
    TileUpdater,
    TransferPayloads,
    TransferredPayloads,
    TransportLogic,
    PayloadTransforms,
}

fn make_layer(
    config: &MapConfig,
    texture: Handle<Image>,
    z: f32,
    name: &'static str,
) -> impl Bundle {
    (
        StateScoped(GameState::FactoryGame),
        Name::new(name),
        TilemapBundle {
            size: config.size,
            tile_size: config.tile_size,
            grid_size: config.grid_size,
            map_type: config.map_type,
            anchor: TilemapAnchor::Center,
            texture: TilemapTexture::Single(texture),
            storage: TileStorage::empty(config.size),
            transform: Transform::from_xyz(0.0, 0.0, z),
            ..default()
        },
    )
}

#[derive(Resource)]
struct MapConfig {
    size: TilemapSize,
    tile_size: TilemapTileSize,
    grid_size: TilemapGridSize,
    map_type: TilemapType,
}

impl Default for MapConfig {
    fn default() -> Self {
        let map_size = TilemapSize { x: 100, y: 100 };
        let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
        let grid_size = tile_size.into();

        Self {
            size: map_size,
            tile_size,
            grid_size,
            map_type: Default::default(),
        }
    }
}

#[derive(Component)]
pub struct BaseLayer;

fn make_base_layer(mut commands: Commands, sprite_sheet: Res<SpriteSheet>, config: Res<MapConfig>) {
    commands.spawn((
        BaseLayer,
        make_layer(&config, sprite_sheet.image(), 0.0, "BaseLayer"),
    ));
}

#[derive(Event)]
pub struct BaseLayerEntityDespawned(TilePos);
