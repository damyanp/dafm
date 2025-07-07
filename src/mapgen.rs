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
use serde::{Deserialize, Serialize};
use tiled::{Loader, PropertyValue};

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
        grid.resize_with(size.count(), || MapEntry::new(&tile_set.tiles));

        Generator {
            size: *size,
            tile_set,
            grid,
        }
    }

    pub fn reset(&mut self) {
        for entry in self.grid.iter_mut() {
            entry.collapsed = false;
            entry.options = self.tile_set.tiles.clone();
        }
    }

    pub fn get(&self) -> Vec<Tile> {
        self.grid
            .iter()
            .enumerate()
            .map(|(index, entry)| Tile {
                pos: self.index_to_pos(index),
                state: if entry.collapsed {
                    TileState::Collapsed(entry.options[0].index)
                } else {
                    TileState::Options(entry.entropy)
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

        self.grid.iter().all(|e| e.collapsed)
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
            .filter(|e| !e.collapsed)
            .map(|e| e.entropy)
            .reduce(f32::min)
            .unwrap_or(f32::MAX);

        let candidates: Vec<usize> = self
            .grid
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.collapsed && e.entropy == min_entropy)
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
        self.grid[index].collapse();
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

                if neighbor.collapsed {
                    continue;
                }

                let changed = neighbor.constrain(&self.tile_set.combos, &tile.options, direction);
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

#[derive(Clone, Copy, PartialEq)]
struct TileOption {
    index: u32,
    weight: f32,
}

struct MapEntry {
    // When collapsed=true there's only one option. Maybe we should just
    // immediately collapsed any with one option and remove the collapsed field?
    collapsed: bool,
    options: Vec<TileOption>,
    entropy: f32,
}

impl MapEntry {
    fn new(tiles: &[TileOption]) -> Self {
        MapEntry {
            collapsed: false,
            options: Vec::from(tiles),
            entropy: calculate_entropy(tiles),
        }
    }

    fn collapse(&mut self) {
        assert!(!self.collapsed);

        if self.options.is_empty() {
            self.collapsed = true;
            self.options = vec![TileOption {
                index: 0,
                weight: 0.0,
            }];
        } else {
            let weighted_index = WeightedIndex::new(self.options.iter().map(|o| o.weight)).unwrap();
            let random_index = weighted_index.sample(&mut rand::rng());
            self.options = vec![self.options[random_index]];
            self.collapsed = true;
        }
    }

    fn constrain(
        &mut self,
        combos: &TileCombos,
        from_options: &[TileOption],
        from_direction: SquareDirection,
    ) -> bool {
        assert!(!self.collapsed);

        let combos = match from_direction {
            SquareDirection::East | SquareDirection::West => &combos.horizontal,
            SquareDirection::North | SquareDirection::South => &combos.vertical,
            _ => panic!("Unexpected direction"),
        };

        let new_options: Vec<TileOption> = self
            .options
            .iter()
            .cloned()
            .filter(|option| {
                combos.iter().any(|[a, b]| match from_direction {
                    SquareDirection::West | SquareDirection::North => {
                        *a == option.index && from_options.iter().any(|o| o.index == *b)
                    }
                    SquareDirection::East | SquareDirection::South => {
                        from_options.iter().any(|o| o.index == *a) && (*b == option.index)
                    }
                    _ => panic!(),
                })
            })
            .collect();

        if new_options.len() != self.options.len() {
            self.entropy = calculate_entropy(&new_options);
            self.options = new_options;
            true
        } else {
            false
        }
    }
}

fn calculate_entropy(tiles: &[TileOption]) -> f32 {
    if tiles.is_empty() {
        return 0.0;
    }

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
    tiles: Vec<TileOption>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct TileCombos {
    horizontal: Vec<[u32; 2]>,
    vertical: Vec<[u32; 2]>,
}

impl TileSet {
    fn load() -> Self {
        let mut loader = Loader::new();

        let tileset = loader.load_tsx_tileset("assets/summerfloor.xml").unwrap();
        info!("Loaded tileset {}", tileset.name);

        let mut combos = TileCombos::default();
        let mut tiles = Vec::new();

        for (a, tile_a) in tileset.tiles() {
            if let Some(a_edges) = get_edges(&tile_a) {
                tiles.push(TileOption {
                    index: a,
                    weight: tile_a.probability,
                });

                for (b, tile_b) in tileset.tiles() {
                    if let Some(b_edges) = get_edges(&tile_b) {
                        if a_edges.right == b_edges.left {
                            combos.horizontal.push([a, b]);
                        }

                        if a_edges.bottom == b_edges.top {
                            combos.vertical.push([a, b]);
                        }
                    }
                }
            }
        }

        println!(
            "{} tiles, {} horizontal combos, {} vertical combos",
            tiles.len(),
            combos.horizontal.len(),
            combos.vertical.len()
        );

        TileSet { combos, tiles }
    }
}

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
