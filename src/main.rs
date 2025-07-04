use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct TileCombos {
    horizontal: Vec<[u32; 2]>,
    vertical: Vec<[u32; 2]>,
}

#[derive(Resource)]
struct TileSetInfo {
    combos: TileCombos,
    tiles: Vec<u32>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(TilemapPlugin)
        .add_systems(Startup, startup)
        .add_systems(Update, update_labels)
        .run();
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    array_texture_loader: Res<ArrayTextureLoader>,
) {
    commands.spawn(Camera2d);

    commands.insert_resource(TileSetInfo::load());

    let texture = asset_server.load("kentangpixel/SummerFloor.png");

    let map_size = TilemapSize { x: 32, y: 32 };

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    // Create text style for coordinate labels
    let text_font = TextFont {
        font_size: 10.0,
        ..default()
    };
    let text_color = TextColor(Color::WHITE);

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(0),
                    ..default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);

            // Calculate world position for the text label
            let world_pos = tile_pos.center_in_world(&map_size, &grid_size, &tile_size, &map_type, &TilemapAnchor::Center);
            
            // Spawn text label showing coordinates
            commands.spawn((
                Text2d::new(format!("{},{}", x, y)),
                text_font.clone(),
                text_color,
                Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
                tile_pos
            ));
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

    array_texture_loader.add(TilemapArrayTexture {
        texture: TilemapTexture::Single(asset_server.load("kentangpixel/SummerFloor.png")),
        tile_size,
        ..default()
    });
}

fn update_labels(mut query: Query<(&mut Text2d, &TilePos)>) {
    for (mut text, tile_pos) in &mut query {
        text.0 = format!("!{}", tile_pos.x);
    }
}

impl TileSetInfo {
    fn load() -> Self {
        // Load and parse the combos.json file
        let combos_json = std::fs::read_to_string("assets/kentangpixel/combos.json")
            .expect("Failed to read combos.json file");
        let combos: TileCombos =
            serde_json::from_str(&combos_json).expect("Failed to parse combos.json");

        let tiles: Vec<u32> = combos
            .horizontal
            .iter()
            .chain(combos.vertical.iter())
            .flat_map(|pair| [pair[0], pair[1]])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        println!(
            "Loaded {} horizontal combos and {} vertical combos out of {} unique ids",
            combos.horizontal.len(),
            combos.vertical.len(),
            tiles.len()
        );

        TileSetInfo {
            combos,
            tiles
        }
    }
}
