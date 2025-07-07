use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

mod mapgen;
mod mapgen_viz;
use mapgen_viz::MapGenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(TilemapPlugin)
        .add_plugins(MapGenPlugin)
        // .add_plugins(map::MapPlugin)
        .add_systems(Startup, startup)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

mod map {
    use crate::mapgen;
    use bevy::prelude::*;
    use bevy_ecs_tilemap::prelude::*;

    pub struct MapPlugin;

    impl Plugin for MapPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, startup);
        }
    }

    fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
        let texture = asset_server.load("kentangpixel/SummerFloor.png");

        let map_size = TilemapSize { x: 32, y: 32 };

        let map = {
            info!("Generating map....");
            let mut generator = mapgen::Generator::new(&map_size);

            loop {
                while !generator.step() {}

                let result = generator.get();
                if result
                    .iter()
                    .all(|t| t.state != mapgen::TileState::Collapsed(0))
                {
                    info!("...done");
                    break result;
                }

                info!("...trying again");
                generator.reset();
            }
        };

        let tilemap_entity = commands.spawn_empty().id();

        let mut tile_storage = TileStorage::empty(map_size);

        let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();

        for x in 0..map_size.x {
            for y in 0..map_size.y {
                let tile_pos = TilePos { x, y };

                let tile = &map[tile_pos.to_index(&map_size)];
                let tile_index = match tile.state {
                    mapgen::TileState::Collapsed(i) => i,
                    mapgen::TileState::Options(_) => 0,
                };

                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(tile_index),
                        ..default()
                    })
                    .id();
                tile_storage.set(&tile_pos, tile_entity);
            }
        }

        commands.entity(tilemap_entity).insert(TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(texture),
            tile_size,
            anchor: TilemapAnchor::Center,
            ..default()
        });
    }
}
