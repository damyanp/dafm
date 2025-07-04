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

fn update(
    mut commands: Commands,
    mut run_step_event: EventReader<RunStepEvent>,
    mut query: Query<(&mut MapGenState, &mut TileTextureIndex, &TilePos, Entity)>,
    new_tile_maps: Query<Entity, (With<TileSetInfo>, Without<MapGeneration>)>,
    tiles: Query<&TilemapId>,
    tile_maps: Query<(&TileStorage, &TileSetInfo, &TilemapSize), With<MapGeneration>>,
) {
    for _ in run_step_event.read() {
        for entity in new_tile_maps.iter() {
            info!("Adding new MapGeneration for {entity:?}");
            commands.entity(entity).insert(MapGeneration {});
        }

        // This assumes there's only one tilemap.  But we could check TilemapId
        // on the tile entities to separate out different tilemaps.

        // Figure out which tile we're going to consider - this will be one
        // chosen randomly from all the ones that have the fewest options.
        //
        // Note: real versions of this use "Shannon Entropy", and take into
        // consideration a weighted "how much we want this type of tile to
        // appear" value.
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
            continue;
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

        while !remaining.is_empty() {
            let tile_entity = remaining.pop().unwrap();
            let (state, _, pos, _) = query.get(tile_entity).unwrap();

            // info!("Updating from {pos:?}");

            let options = state.options.clone();
            let neighbors = Neighbors::get_square_neighboring_positions(pos, map_size, false)
                .entities(tile_storage);

            for (direction, neighbor) in neighbors.iter_with_direction() {
                if let Ok((mut neighbor_state, _, neighbor_pos, _)) = query.get_mut(*neighbor) {
                    if !neighbor_state.collapsed {
                        // info!("  neighbor {direction:?} {neighbor_pos:?}");
                        let changed =
                            neighbor_state.constrain(&tile_set_info.combos, &options, direction);

                        if changed {
                            remaining.push(*neighbor);
                        }
                    }
                }
            }
        }
    }
}

impl MapGenState {
    fn constrain(
        &mut self,
        combos: &TileCombos,
        from_options: &Vec<u32>,
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
                // info!("   checking {option}");
                combos.iter().any(|[a, b]| {
                    let result = match from_direction {
                        SquareDirection::West | SquareDirection::North => {
                            a == option && from_options.contains(b)
                        }
                        SquareDirection::East | SquareDirection::South => {
                            from_options.contains(a) && (b == option)
                        }
                        _ => panic!(),
                    };

                    if result {
                        // info!("     - match for {a},{b}");
                    }
                    result
                })
            })
            .collect();

        if new_options.is_empty() {
            // info!("{:?} couldn't find any options for {:?} {:?}", self.options, from_options, from_direction);
        }

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
                .insert(MapGenState::new(set_info, label));
        }
    }
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
