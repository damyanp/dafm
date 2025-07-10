use std::collections::HashMap;

use bevy::{log::info, math::ops::ln};
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    map::TilemapSize,
    tiles::TilePos,
};
use rand::{
    Rng,
    distr::{Distribution, Uniform, weighted::WeightedIndex},
};
use tiled::{Loader, Tileset, WangId};

// Based on Wave Function Collapse Algorithm.  See for example:
//   https://robertheaton.com/2018/12/17/wavefunction-collapse-algorithm/

pub struct Generator {
    size: TilemapSize,
    tile_set: TileSet,
    grid: Vec<MapEntry>,
}

pub struct Tile {
    pub pos: TilePos,
    pub state: TileState,
}

#[derive(PartialEq)]
pub enum TileState {
    Collapsed(u32),
    Options(f32),
}

impl Generator {
    pub fn new(size: &TilemapSize) -> Self {
        let tile_set = TileSet::load();

        let mut grid = Vec::new();
        grid.resize_with(size.count(), || {
            MapEntry::new(tile_set.classes.get_tile_classes_options())
        });

        Generator {
            size: *size,
            tile_set,
            grid,
        }
    }

    pub fn reset(&mut self) {
        for entry in self.grid.iter_mut() {
            *entry = MapEntry::new(self.tile_set.classes.get_tile_classes_options())
        }
    }

    pub fn get(&self) -> Vec<Tile> {
        self.grid
            .iter()
            .enumerate()
            .map(|(index, entry)| Tile {
                pos: self.index_to_pos(index),
                state: match *entry {
                    MapEntry::Collapsed { index, .. } => TileState::Collapsed(index),
                    MapEntry::Superposition { entropy, .. } => TileState::Options(entropy),
                },
            })
            .collect()
    }

    pub fn step(&mut self) -> bool {
        let Some(tile) = self.pick_tile_to_update() else {
            // No tiles to update: we're done!
            return true;
        };

        self.collapse(tile);
        self.update_options(tile);

        self.grid.iter().all(|e| e.is_collapsed())
    }

    fn pick_tile_to_update(&self) -> Option<TilePos> {
        // Figure out which tile we're going to consider - this will be one
        // chosen randomly from all the ones that have the fewest options.
        //
        // Note: real versions of this use "Shannon Entropy", and take into
        // consideration a weighted "how much we want this type of tile to
        // appear" value.
        let min_entropy = self
            .grid
            .iter()
            .filter_map(|e| match *e {
                MapEntry::Superposition { entropy, .. } => Some(entropy),
                _ => None,
            })
            .reduce(f32::min)
            .unwrap_or(f32::MAX);

        let candidates: Vec<usize> = self
            .grid
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if let MapEntry::Superposition { entropy, .. } = **e {
                    entropy <= min_entropy
                } else {
                    false
                }
            })
            .map(|(i, _)| i)
            .collect();

        if candidates.is_empty() {
            return None;
        }

        let index = candidates[rand::rng().random_range(0..candidates.len())];
        Some(self.index_to_pos(index))
    }

    fn collapse(&mut self, tile: TilePos) {
        let index = tile.to_index(&self.size);
        self.grid[index].collapse(&self.tile_set);
    }

    fn update_options(&mut self, starting_tile: TilePos) {
        let mut remaining = vec![starting_tile];

        while let Some(tile_pos) = remaining.pop() {
            let neighbors =
                Neighbors::get_square_neighboring_positions(&tile_pos, &self.size, false);
            for (direction, neighbor_pos) in neighbors.iter_with_direction() {
                // Slight dance to get multiple references from the vector,
                // where neighbor is mutable.
                let [tile, neighbor] = self
                    .grid
                    .get_disjoint_mut([
                        tile_pos.to_index(&self.size),
                        neighbor_pos.to_index(&self.size),
                    ])
                    .expect("tile_pos != neighbor_pos");

                if neighbor.is_collapsed() {
                    continue;
                }

                // println!("Constrain {neighbor_pos:?}");
                let changed = neighbor.constrain(&self.tile_set, tile, direction);
                if changed {
                    remaining.push(*neighbor_pos);
                }
            }
        }
    }

    fn index_to_pos(&self, index: usize) -> TilePos {
        let index = u32::try_from(index).unwrap();
        TilePos {
            x: index % self.size.x,
            y: index / self.size.x,
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
struct TileOption {
    class: TileClass,
    index: u32,
}

enum MapEntry {
    Collapsed {
        class: TileClass,
        index: u32,
    },
    Superposition {
        options: Vec<TileClass>,
        entropy: f32,
    },
}

impl MapEntry {
    fn new(options: Vec<TileClass>) -> Self {
        MapEntry::Superposition {
            options: options,
            entropy: f32::MAX,
        }
    }

    fn is_collapsed(&self) -> bool {
        matches!(self, MapEntry::Collapsed { .. })
    }

    fn collapse(&mut self, tile_set: &TileSet) {
        *self = match self {
            MapEntry::Superposition { options, .. } => {
                if options.is_empty() {
                    MapEntry::Collapsed {
                        class: TileClass(0),
                        index: 0,
                    }
                } else {
                    // Pick a class
                    let weights = options
                        .iter()
                        .map(|class| tile_set.classes.get_weight(class));

                    let weighted_index = WeightedIndex::new(weights).unwrap();
                    let class = &options[weighted_index.sample(&mut rand::rng())];

                    // Now pick a tile of that class
                    let tiles: Vec<u32> = tile_set
                        .tiles
                        .iter()
                        .filter(|t| t.class == *class)
                        .map(|t| t.index)
                        .collect();

                    let index = tiles[Uniform::new(0, tiles.len())
                        .unwrap()
                        .sample(&mut rand::rng())];

                    MapEntry::Collapsed {
                        class: *class,
                        index,
                    }
                }
            }
            _ => return,
        };
    }

    fn constrain(
        &mut self,
        tile_set: &TileSet,
        from_tile: &MapEntry,
        from_direction: SquareDirection,
    ) -> bool {
        match self {
            MapEntry::Collapsed { .. } => panic!(),
            MapEntry::Superposition { options, entropy } => {
                let old_len = options.len();

                options.retain(|option| {
                    tile_set
                        .combos
                        .is_allowed(&from_direction, from_tile, option)
                });

                if options.len() != old_len {
                    // println!(" Changed: {} --> {}", old_len, self.options.len());
                    *entropy = calculate_entropy(options, &tile_set);
                    true
                } else {
                    false
                }
            }
        }
    }
}

fn calculate_entropy(options: &[TileClass], tile_set: &TileSet) -> f32 {
    if options.is_empty() {
        return 0.0;
    }

    let weights: Vec<f32> = options
        .iter()
        .map(|class| tile_set.classes.get_weight(class))
        .collect();

    let sum_weights = weights.iter().cloned().reduce(|acc, e| acc + e).unwrap();

    let sum_weight_logs = weights
        .iter()
        .cloned()
        .reduce(|acc, e| acc + e * ln(e))
        .unwrap();

    ln(sum_weights) - (sum_weight_logs / sum_weights)
}

struct TileSet {
    combos: TileCombos,
    classes: TileClasses,
    tiles: Vec<TileOption>,
}

struct TileCombos {
    horizontal: Combos,
    vertical: Combos,
}

impl TileCombos {
    fn is_allowed(
        &self,
        from_direction: &SquareDirection,
        from_tile: &MapEntry,
        option: &TileClass,
    ) -> bool {
        let combos = match from_direction {
            SquareDirection::East | SquareDirection::West => &self.horizontal,
            SquareDirection::North | SquareDirection::South => &self.vertical,
            _ => panic!("Unexpected direction"),
        };

        let reversed = matches!(
            from_direction,
            SquareDirection::East | SquareDirection::South
        );

        combos.is_allowed(from_tile, option, reversed)
    }
}

struct Combos {
    combos: Vec<[TileClass; 2]>,
    reversed_combos: Vec<[TileClass; 2]>,
}

impl Combos {
    fn new(combos: &[[TileClass; 2]]) -> Self {
        let mut reversed_combos: Vec<[TileClass; 2]> =
            combos.iter().map(|[a, b]| [*b, *a]).collect();
        let mut combos = combos.to_owned();

        combos.sort();
        reversed_combos.sort();

        Combos {
            combos,
            reversed_combos,
        }
    }

    fn is_allowed(&self, from_tile: &MapEntry, option: &TileClass, reversed: bool) -> bool {
        let combos = if reversed {
            &self.reversed_combos
        } else {
            &self.combos
        };

        // println!("  {from_options:?} {option:?} {reversed}:");

        let mut index = combos.partition_point(|[a, _]| a < option);
        while index < combos.len() {
            let combo = &combos[index];
            if &combo[0] != option {
                break;
            }
            // println!("    check [{},{}]", combo[0], combo[1]);

            match from_tile {
                MapEntry::Collapsed { class, .. } => {
                    if *class == combo[1] {
                        return true;
                    }
                }
                MapEntry::Superposition { options, .. } => {
                    if options.binary_search_by(|o| o.cmp(&combo[1])).is_ok() {
                        return true;
                    }
                }
            }

            index += 1;
        }
        false
    }
}

struct TileClasses {
    classes: Vec<TileClassData>,
}

#[derive(Debug)]
struct TileClassData {
    wang_id: WangId,
    tiles: Vec<u32>,
    weight: f32,
}

impl TileClassData {
    fn connects_with(&self, other: &TileClassData) -> (bool, bool) {
        let a = &self.wang_id.0;
        let right = [a[1], a[2], a[3]];
        let bottom = [a[5], a[4], a[3]];

        let b = &other.wang_id.0;
        let left = [b[7], b[6], b[5]];
        let top = [b[7], b[0], b[1]];

        (left == right, bottom == top)
    }
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
struct TileClass(usize);

impl TileClasses {
    fn new(tileset: &Tileset) -> Self {
        let mut classes: HashMap<[u8; 8], TileClassData> = HashMap::default();

        for (tile_index, wang_tile) in tileset.wang_sets[0].wang_tiles.iter() {
            let id = wang_tile.wang_id.0;
            if classes.contains_key(&id) {
                classes.get_mut(&id).unwrap().tiles.push(*tile_index);
            } else {
                classes.insert(
                    id.clone(),
                    TileClassData {
                        wang_id: WangId(id),
                        tiles: vec![*tile_index],
                        weight: get_wang_tile_weight(tileset, &wang_tile.wang_id),
                    },
                );
            }
        }

        let mut classes: Vec<TileClassData> = classes.into_values().collect();
        classes.sort_by(|a, b| a.wang_id.0.cmp(&b.wang_id.0));

        println!("{classes:?}");

        TileClasses { classes }
    }

    fn len(&self) -> usize {
        self.classes.len()
    }

    fn iter(&self) -> impl Iterator<Item = (TileClass, &TileClassData)> {
        self.classes
            .iter()
            .enumerate()
            .map(|(index, class)| (TileClass(index), class))
    }

    fn get_tile_classes_options(&self) -> Vec<TileClass> {
        (0..self.classes.len())
            .into_iter()
            .map(|id| TileClass(id))
            .collect()
    }

    fn get_weight(&self, class: &TileClass) -> f32 {
        self.classes[class.0].weight
    }
}

/// Wangtile weight is calculated as the product of the probabilities of all the
/// WangColors in the tile.
fn get_wang_tile_weight(tileset: &Tileset, wang_tile: &WangId) -> f32 {
    let mut ids = Vec::from(wang_tile.0);
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .filter(|id| id != &0)
        .map(|id| tileset.wang_sets[0].wang_colors[(id as usize) - 1].probability)
        .reduce(|a, e| a * e)
        .unwrap()
}

impl TileSet {
    fn load() -> Self {
        let mut loader = Loader::new();

        let tileset = loader.load_tsx_tileset("assets/summerfloor.xml").unwrap();
        info!("Loaded tileset {}", tileset.name);

        let classes = TileClasses::new(&tileset);

        let mut tiles = Vec::new();
        for (class, tile_class_data) in classes.iter() {
            for index in &tile_class_data.tiles {
                tiles.push(TileOption {
                    class,
                    index: index.clone(),
                });
            }
        }

        tiles.sort_by(|a, b| a.index.cmp(&b.index));

        let mut horizontal = Vec::new();
        let mut vertical = Vec::new();

        for (a, a_data) in classes.iter() {
            for (b, b_data) in classes.iter() {
                let (connects_horizontally, connects_vertically) = a_data.connects_with(&b_data);

                if connects_horizontally {
                    horizontal.push([a, b]);
                }

                if connects_vertically {
                    vertical.push([a, b]);
                }
            }
        }

        println!(
            "{} tiles, {} classes {} horizontal combos, {} vertical combos",
            tiles.len(),
            classes.len(),
            horizontal.len(),
            vertical.len()
        );

        let combos = TileCombos {
            horizontal: Combos::new(&horizontal),
            vertical: Combos::new(&vertical),
        };

        TileSet {
            combos,
            classes,
            tiles,
        }
    }
}
