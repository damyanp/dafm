use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::GameState;

mod conveyor;
mod conveyor_belts;
mod dev;
mod distributor;
mod generator;
mod helpers;
mod interaction;
mod sink;

use helpers::*;

pub struct FactoryGamePlugin;
impl Plugin for FactoryGamePlugin {
    fn build(&self, app: &mut App) {
        app //
            .add_plugins(conveyor_belts::ConveyorBeltsPlugin)
            .add_plugins(conveyor::PayloadPlugin)
            .add_plugins(dev::DevPlugin)
            .add_plugins(distributor::DistributorPlugin)
            .add_plugins(generator::GeneratorPlugin)
            .add_plugins(interaction::ConveyorInteractionPlugin)
            .add_plugins(sink::SinkPlugin)
            .insert_resource(MapConfig::default())
            .add_event::<BaseLayerEntityDespawned>()
            .configure_sets(
                Update,
                (
                    ConveyorSystems::TileGenerator,
                    ConveyorSystems::TileUpdater,
                    ConveyorSystems::TransportLogic,
                    ConveyorSystems::PayloadTransforms,
                )
                    .chain()
                    .run_if(in_state(GameState::FactoryGame)),
            )
            .add_systems(OnEnter(GameState::FactoryGame), make_base_layer);
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum ConveyorSystems {
    TileGenerator,
    TileUpdater,
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

fn make_base_layer(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<MapConfig>) {
    let texture = asset_server.load("sprites.png");
    commands.spawn((BaseLayer, make_layer(&config, texture, 0.0, "BaseLayer")));
}

#[derive(Event)]
pub struct BaseLayerEntityDespawned(TilePos);
