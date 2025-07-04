use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use serde::{Deserialize, Serialize};

pub struct MapGenPlugin;

#[derive(Event)]
pub struct RunStepEvent;

impl Plugin for MapGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_event::<RunStepEvent>();
        app.add_systems(
            Update,
            (initialize_map_generation, update, update_labels).chain(),
        );
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("kentangpixel/SummerFloor.png");

    let map_size = TilemapSize { x: 32, y: 32 };

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

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
        }
    }

    commands
        .entity(tilemap_entity)
        .insert(TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(texture),
            tile_size,
            anchor: TilemapAnchor::Center,
            ..default()
        })
        .insert(TileSetInfo::load());
}

#[derive(Debug, Deserialize, Serialize)]
struct TileCombos {
    horizontal: Vec<[u32; 2]>,
    vertical: Vec<[u32; 2]>,
}

#[allow(dead_code)]
#[derive(Component)]
struct TileSetInfo {
    combos: TileCombos,
    tiles: Vec<u32>,
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

        TileSetInfo { combos, tiles }
    }
}

#[derive(Component)]
struct MapGeneration {}

impl MapGeneration {
    fn new(
        commands: &mut Commands,
        map_size: &TilemapSize,
        grid_size: &TilemapGridSize,
        tile_size: &TilemapTileSize,
        map_type: &TilemapType,
    ) -> Self {
        // Add labels
        for y in 0..map_size.y {
            for x in 0..map_size.x {
                let tile_pos = TilePos { x, y };

                let world_pos = tile_pos.center_in_world(
                    map_size,
                    grid_size,
                    tile_size,
                    map_type,
                    &TilemapAnchor::Center,
                );
            }
        }

        MapGeneration {}

        // let text_font = TextFont {
        //     font_size: 10.0,
        //     ..default()
        // };
        // let text_color = TextColor(Color::WHITE);

        // // Calculate world position for the text label
        // let world_pos = tile_pos.center_in_world(&map_size, &grid_size, &tile_size, &map_type, &TilemapAnchor::Center);

        // // Spawn text label showing coordinates
        // commands.spawn((
        //     Text2d::new(format!("{},{}", x, y)),
        //     text_font.clone(),
        //     text_color,
        //     Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
        //     tile_pos
        // ));
    }
}

fn update(
    mut commands: Commands,
    mut run_step_event: EventReader<RunStepEvent>,
    mut tile_maps: Query<(
        Entity,
        &TileSetInfo,
        &TileStorage,
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &MapGeneration,
    )>,
    new_tile_maps: Query<Entity, (With<TileSetInfo>, Without<MapGeneration>)>,
) {
    for _ in run_step_event.read() {
        for entity in new_tile_maps.iter() {
            info!("Adding new MapGeneration for {entity:?}");
            commands.entity(entity).insert(MapGeneration {});
        }

        for (entity, set_info, storage, map_size, grid_size, tile_size, map_type, generation) in
            tile_maps.iter_mut()
        {
            info!("Run step for {:?}", entity);
        }
    }
}

#[derive(Component)]
struct MapGenOptions {
    options: Vec<u32>,
    label: Entity,
}

impl MapGenOptions {
    fn new(tile_set_info: &TileSetInfo, label: Entity) -> Self {
        MapGenOptions {
            options: tile_set_info.tiles.clone(),
            label,
        }
    }
}

fn initialize_map_generation(
    mut commands: Commands,
    tile_maps: Query<
        (
            &TileSetInfo,
            &TileStorage,
            &Transform,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
            &MapGeneration,
        ),
        Added<MapGeneration>,
    >,
    tiles: Query<&TilePos>,
) {
    for (
        set_info, //
        storage,
        transform,
        map_size,
        grid_size,
        tile_size,
        map_type,
        anchor,
        _,
    ) in tile_maps
    {
        info!("Initializing MapGeneration");

        for tile_entity in storage.iter().flatten() {
            let tile_pos = tiles.get(*tile_entity).unwrap();
            let tile_center = tile_pos
                .center_in_world(map_size, grid_size, tile_size, map_type, anchor)
                .extend(1.0);
            let transform = *transform * Transform::from_translation(tile_center);

            let label = commands
                .spawn((
                    Text2d::new("-"),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextLayout::new_with_justify(JustifyText::Center),
                    transform,
                ))
                .id();

            commands
                .entity(*tile_entity)
                .insert(MapGenOptions::new(&set_info, label));
        }
    }
}

fn update_labels(mut labels: Query<&mut Text2d>, map_gens: Query<&MapGenOptions>) {
    for map_gen_options in map_gens.iter() {
        let mut label = labels.get_mut(map_gen_options.label).unwrap();
        label.0 = format!("{}", map_gen_options.options.len());
    }
}
