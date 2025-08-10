use crate::GameState;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

mod helpers;
use helpers::*;

mod interaction;
use interaction::InteractionPlugin;
use interaction::*;

mod visuals;
use visuals::*;

mod dev;

pub struct ConveyorPlugin;
impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InteractionPlugin)
            .add_plugins(Visuals)
            .add_plugins(dev::Dev)
            .register_type::<Conveyor>()
            .insert_resource(MapConfig::default());
    }
}

#[derive(Component, Clone, Debug, Reflect, Default)]
struct Conveyor(ConveyorDirection);

fn make_layer(
    config: &MapConfig,
    texture: Handle<Image>,
    z: f32,
    name: &'static str,
) -> impl Bundle {
    (
        StateScoped(GameState::Conveyor),
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
