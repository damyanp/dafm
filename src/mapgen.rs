use std::collections::HashMap;

use bevy::{log::info, math::ops::ln};
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    map::TilemapSize,
    tiles::TilePos,
};
use rand::{
    Rng,
    distr::{Distribution, weighted::WeightedIndex},
};
use tiled::{Loader, PropertyValue, Tileset};

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
        grid.resize_with(size.count(), || MapEntry::new(&tile_set.classes));

        Generator {
            size: *size,
            tile_set,
            grid,
        }
    }

    pub fn reset(&mut self) {
        for entry in self.grid.iter_mut() {
            *entry = MapEntry::new(&self.tile_set.classes)
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
        self.grid[index].collapse(&self.tile_set.tiles);
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
    weight: f32,
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
    fn new(options: &[TileClass]) -> Self {
        MapEntry::Superposition {
            options: options.to_owned(),
            entropy: f32::MAX,
        }
    }

    fn is_collapsed(&self) -> bool {
        matches!(self, MapEntry::Collapsed { .. })
    }

    fn collapse(&mut self, tiles: &[TileOption]) {
        *self = match self {
            MapEntry::Superposition { options, .. } => {
                if options.is_empty() {
                    MapEntry::Collapsed {
                        class: TileClass(0),
                        index: 0,
                    }
                } else {
                    let tiles: Vec<&TileOption> = tiles
                        .iter()
                        .filter(|tile| options.contains(&tile.class))
                        .collect();

                    let weighted_index =
                        WeightedIndex::new(tiles.iter().map(|tile| tile.weight)).unwrap();
                    let option = &tiles[weighted_index.sample(&mut rand::rng())];
                    MapEntry::Collapsed {
                        class: option.class,
                        index: option.index,
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
                    *entropy = calculate_entropy(options, &tile_set.tiles);
                    true
                } else {
                    false
                }
            }
        }
    }
}

fn calculate_entropy(options: &[TileClass], tiles: &[TileOption]) -> f32 {
    if options.is_empty() {
        return 0.0;
    }

    let tiles: Vec<&TileOption> = tiles
        .iter()
        .filter(|tile| options.contains(&tile.class))
        .collect();

    let sum_weights = tiles
        .iter()
        .map(|e| e.weight)
        .reduce(|acc, e| acc + e)
        .unwrap();

    let sum_weight_logs = tiles
        .iter()
        .map(|e| e.weight)
        .reduce(|acc, e| acc + e * ln(e))
        .unwrap();

    ln(sum_weights) - (sum_weight_logs / sum_weights)
}

struct TileSet {
    combos: TileCombos,
    classes: Vec<TileClass>,
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

struct TileClassData {
    edges: Edges,
    tiles: Vec<u32>,
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
struct TileClass(usize);

impl TileClasses {
    fn new(tileset: &Tileset) -> Self {
        let mut classes: HashMap<Edges, TileClassData> = HashMap::default();

        for (tile_index, tile) in tileset.tiles() {
            if let Some(edges) = get_edges(&tile) {
                if classes.contains_key(&edges) {
                    classes.get_mut(&edges).unwrap().tiles.push(tile_index);
                } else {
                    classes.insert(
                        edges.clone(),
                        TileClassData {
                            edges,
                            tiles: vec![tile_index],
                        },
                    );
                }
            }
        }

        let mut classes: Vec<TileClassData> = classes.into_values().collect();
        classes.sort_by(|a, b| a.edges.cmp(&b.edges));

        TileClasses { classes }
    }

    fn len(&self) -> usize {
        self.classes.len()
    }

    fn iter(&self) -> impl Iterator<Item = (TileClass, &Edges)> {
        self.classes
            .iter()
            .enumerate()
            .map(|(index, class)| (TileClass(index), &class.edges))
    }

    fn get(&self, edges: &Edges) -> TileClass {
        TileClass(
            self.classes
                .binary_search_by(|a| a.edges.cmp(edges))
                .unwrap(),
        )
    }
}

impl TileSet {
    fn load() -> Self {
        let mut loader = Loader::new();

        let tileset = loader.load_tsx_tileset("assets/summerfloor.xml").unwrap();
        info!("Loaded tileset {}", tileset.name);

        let classes = TileClasses::new(&tileset);

        let mut tiles: Vec<TileOption> = tileset
            .tiles()
            .flat_map(|(index, tile)| {
                get_edges(&tile).map(|edges| TileOption {
                    class: classes.get(&edges),
                    index,
                    weight: tile.probability,
                })
            })
            .collect();
        tiles.sort_by(|a, b| a.index.cmp(&b.index));

        let mut horizontal = Vec::new();
        let mut vertical = Vec::new();

        for (a, a_edges) in classes.iter() {
            for (b, b_edges) in classes.iter() {
                if a_edges.right == b_edges.left {
                    horizontal.push([a, b]);
                }

                if a_edges.bottom == b_edges.top {
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

        let classes = classes.iter().map(|(c, _)| c).collect();

        TileSet {
            combos,
            classes,
            tiles,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, PartialOrd, Ord)]
struct Edges {
    top: [char; 2],
    right: [char; 2],
    bottom: [char; 2],
    left: [char; 2],
}

fn get_edges(tile: &tiled::Tile) -> Option<Edges> {
    if let Some(PropertyValue::StringValue(edges)) = tile.properties.get("edges") {
        if edges != "????" {
            return Some(Edges::from_edges(edges));
        }
    }
    if let Some(PropertyValue::StringValue(submat)) = tile.properties.get("submat") {
        if submat != "????" {
            return Some(Edges::from_submat(submat));
        }
    }
    None
}

impl Edges {
    fn from_submat(s: &str) -> Self {
        let s: Vec<char> = s.chars().collect();

        Edges {
            top: [s[0], s[1]],
            right: [s[1], s[3]],
            bottom: [s[2], s[3]],
            left: [s[0], s[2]],
        }
    }

    fn from_edges(s: &str) -> Self {
        let s: Vec<char> = s.chars().collect();
        Edges {
            top: [s[0], s[1]],
            right: [s[2], s[3]],
            bottom: [s[4], s[5]],
            left: [s[6], s[7]],
        }
    }
}
