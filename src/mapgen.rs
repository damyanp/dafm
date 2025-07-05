use std::ops::ControlFlow;

use bevy::prelude::*;
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    prelude::*,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

// https://robertheaton.com/2018/12/17/wavefunction-collapse-algorithm/

pub struct MapGenPlugin;

#[derive(Event)]
pub struct RunStepEvent;

#[derive(Event)]
pub struct AutoBuildEvent;

#[derive(Event)]
pub struct ResetEvent;

impl Plugin for MapGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_event::<AutoBuildEvent>()
            .add_event::<RunStepEvent>()
            .add_event::<ResetEvent>()
            .insert_resource(AutoBuild(false))
            .add_systems(
                Update,
                (
                    auto_build,
                    reset_map_generation,
                    initialize_map_generation,
                    run_steps,
                    update_labels,
                )
                    .chain(),
            );
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("kentangpixel/SummerFloor.png");

    let map_size = TilemapSize { x: 16, y: 16 };

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
struct MapGeneration;

#[derive(Resource)]
struct AutoBuild(bool);

fn run_steps(
    mut commands: Commands,
    mut run_step_event: EventReader<RunStepEvent>,
    auto_build: Res<AutoBuild>,
    mut query: Query<(&mut MapGenState, &mut TileTextureIndex, &TilePos, Entity)>,
    new_tile_maps: Query<Entity, (With<TileSetInfo>, Without<MapGeneration>)>,
    tiles: Query<&TilemapId>,
    tile_maps: Query<(&TileStorage, &TileSetInfo, &TilemapSize), With<MapGeneration>>,
    mut reset_event: EventWriter<ResetEvent>,
) {
    if run_step_event.is_empty() && auto_build.0 {
        generation_step(
            &mut commands,
            &mut query,
            new_tile_maps,
            tiles,
            tile_maps,
            &mut reset_event,
        );
        return;
    }
    
    for _ in run_step_event.read() {
        generation_step(
            &mut commands,
            &mut query,
            new_tile_maps,
            tiles,
            tile_maps,
            &mut reset_event,
        );
    }
}

fn generation_step(
    commands: &mut Commands,
    query: &mut Query<(&mut MapGenState, &mut TileTextureIndex, &TilePos, Entity)>,
    new_tile_maps: Query<Entity, (With<TileSetInfo>, Without<MapGeneration>)>,
    tiles: Query<&TilemapId>,
    tile_maps: Query<(&TileStorage, &TileSetInfo, &TilemapSize), With<MapGeneration>>,
    reset_event: &mut EventWriter<ResetEvent>,
) {
    for entity in new_tile_maps.iter() {
        commands.entity(entity).insert(MapGeneration {});
    }

    // This assumes there's only one tilemap.  But we could check TilemapId on
    // the tile entities to separate out different tilemaps.

    // Figure out which tile we're going to consider - this will be one chosen
    // randomly from all the ones that have the fewest options.
    //
    // Note: real versions of this use "Shannon Entropy", and take into
    // consideration a weighted "how much we want this type of tile to appear"
    // value.
    let min_entropy = query
        .iter()
        .filter(|(s, _, _, _)| !s.collapsed)
        .map(|(s, _, _, _)| s.options.len())
        .min()
        .unwrap_or(usize::MAX);

    let candidates: Vec<_> = query
        .iter()
        .filter(|(s, _, _, _)| !s.collapsed && s.options.len() == min_entropy)
        .map(|(_, _, _, e)| e)
        .collect();

    if candidates.is_empty() {
        return;
    }

    // "Collapse" this entity
    let selected_entity = candidates[rand::rng().random_range(0..candidates.len())];
    {
        let (mut state, mut texture_index, _, _) = query.get_mut(selected_entity).unwrap();

        let random_index = state.options[rand::rng().random_range(0..state.options.len())];
        texture_index.0 = random_index;

        if let Some(label) = state.label {
            commands.entity(label).despawn();
            state.label = None;
        }
        state.options = vec![random_index];
        state.collapsed = true;
    }

    // Update options for neighbors (and their neighbors etc.)
    let tile_map_id = tiles.get(selected_entity).unwrap();
    let (tile_storage, tile_set_info, map_size) = tile_maps.get(tile_map_id.0).unwrap();

    let mut remaining = vec![selected_entity];

    while let Some(tile_entity) = remaining.pop() {
        let (state, _, pos, _) = query.get(tile_entity).unwrap();

        let options = state.options.clone();
        let neighbors = Neighbors::get_square_neighboring_positions(pos, map_size, false)
            .entities(tile_storage);

        for (direction, neighbor) in neighbors.iter_with_direction() {
            if let Ok((mut neighbor_state, _, _, _)) = query.get_mut(*neighbor) {
                if !neighbor_state.collapsed {
                    let changed =
                        neighbor_state.constrain(&tile_set_info.combos, &options, direction);

                    if changed {
                        remaining.push(*neighbor);
                    }
                }
            }
        }
    }
    // This assumes there's only one tilemap.  But we could check TilemapId
    // on the tile entities to separate out different tilemaps.

    // Figure out which tile we're going to consider - this will be one
    // chosen randomly from all the ones that have the fewest options.
    //
    // Note: real versions of this use "Shannon Entropy", and take into
    // consideration a weighted "how much we want this type of tile to
    // appear" value.

    // "Collapse" this entity

    // Update options for neighbors (and their neighbors etc.)

    if query.iter().all(|(s, _, _, _)| s.collapsed) {
        // we're done!
        reset_event.write(ResetEvent);
    }
}

impl MapGenState {
    fn constrain(
        &mut self,
        combos: &TileCombos,
        from_options: &[u32],
        from_direction: SquareDirection,
    ) -> bool {
        assert!(!self.collapsed);

        let combos = match from_direction {
            SquareDirection::East | SquareDirection::West => &combos.horizontal,
            SquareDirection::North | SquareDirection::South => &combos.vertical,
            _ => panic!("Unexpected direction"),
        };

        let new_options: Vec<u32> = self
            .options
            .iter()
            .cloned()
            .filter(|option| {
                combos.iter().any(|[a, b]| match from_direction {
                    SquareDirection::West | SquareDirection::North => {
                        a == option && from_options.contains(b)
                    }
                    SquareDirection::East | SquareDirection::South => {
                        from_options.contains(a) && (b == option)
                    }
                    _ => panic!(),
                })
            })
            .collect();

        if new_options.len() != self.options.len() {
            self.options = new_options;
            true
        } else {
            false
        }
    }
}

#[derive(Component)]
struct MapGenState {
    collapsed: bool,
    options: Vec<u32>,
    label: Option<Entity>,
}

impl MapGenState {
    fn new(tile_set_info: &TileSetInfo, label: Entity) -> Self {
        MapGenState {
            collapsed: false,
            options: tile_set_info.tiles.clone(),
            label: Some(label),
        }
    }
}

#[allow(clippy::type_complexity)]
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
                .insert(MapGenState::new(set_info, label));
        }
    }
}

fn reset_map_generation(
    mut commands: Commands,
    mut event: EventReader<ResetEvent>,
    tile_maps: Query<(Entity, &MapGenState)>,
    map_generate: Query<Entity, With<MapGeneration>>,
    mut auto_build: ResMut<AutoBuild>,
) {
    if event.is_empty() {
        return;
    }
    event.clear();

    for (entity, state) in tile_maps {
        if let Some(label) = state.label {
            commands.entity(label).despawn();
        }
        commands.entity(entity).remove::<MapGenState>();
    }

    for m in map_generate {
        commands.entity(m).remove::<MapGeneration>();
    }

    auto_build.0 = false;
}

fn update_labels(mut labels: Query<&mut Text2d>, states: Query<&MapGenState>) {
    for state in states.iter() {
        if let Some(label) = state.label {
            let mut label = labels.get_mut(label).unwrap();
            if !state.collapsed {
                label.0 = format!("{}", state.options.len());
            } else {
                label.0 = format!(
                    "[{}]",
                    if state.options.is_empty() {
                        0
                    } else {
                        state.options[0]
                    }
                );
            }
        }
    }
}

fn auto_build(
    mut commands: Commands,
    mut auto_build_event: EventReader<AutoBuildEvent>,
    mut tile_texture_indices: Query<&mut TileTextureIndex>,
    mut auto_build: ResMut<AutoBuild>,
) {
    if !auto_build_event.is_empty() && !auto_build.0 {
        auto_build.0 = true;

        for mut t in &mut tile_texture_indices {
            t.0 = 0;
        }
    }
}
