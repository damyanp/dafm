use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    map::TilemapSize,
    tiles::TilePos,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

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

pub enum TileState {
    Collapsed(u32),
    Options(usize),
}

impl Generator {
    pub fn new(size: TilemapSize) -> Self {
        let tile_set = TileSet::load();

        let mut grid = Vec::new();
        grid.resize_with(size.count(), || MapEntry::new(&tile_set.tiles));

        Generator {
            size,
            tile_set,
            grid,
        }
    }

    pub fn get(&self) -> Vec<Tile> {
        self.grid
            .iter()
            .enumerate()
            .map(|(index, entry)| Tile {
                pos: self.index_to_pos(index),
                state: if entry.collapsed {
                    TileState::Collapsed(entry.options[0])
                } else {
                    TileState::Options(entry.options.len())
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
            .map(|e| e.options.len())
            .min()
            .unwrap_or(usize::MAX);

        let candidates: Vec<usize> = self
            .grid
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.collapsed && e.options.len() == min_entropy)
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
            y: index / self.size.y,
        }
    }
}

struct MapEntry {
    // When collapsed=true there's only one option. Maybe we should just
    // immediately collapsed any with one option and remove the collapsed field?
    collapsed: bool,
    options: Vec<u32>,
}

impl MapEntry {
    fn new(tiles: &Vec<u32>) -> Self {
        MapEntry {
            collapsed: false,
            options: tiles.clone(),
        }
    }

    fn collapse(&mut self) {
        assert!(!self.collapsed);

        let random_index = rand::rng().random_range(0..self.options.len());
        self.options = vec![self.options[random_index]];
        self.collapsed = true;
    }

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

struct TileSet {
    combos: TileCombos,
    tiles: Vec<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TileCombos {
    horizontal: Vec<[u32; 2]>,
    vertical: Vec<[u32; 2]>,
}

impl TileSet {
    fn load() -> Self {
        // Load and parse the combos.json file
        let combos_json = std::fs::read_to_string("assets/kentangpixel/combos.json")
            .expect("Failed to read combos.json file");
        let combos: TileCombos =
            serde_json::from_str(&combos_json).expect("Failed to parse combos.json");

        // Infer all the possible tiles from those mentioned in the list of
        // valid combinations
        let tiles: Vec<u32> = combos
            .horizontal
            .iter()
            .chain(combos.vertical.iter())
            .flat_map(|pair| [pair[0], pair[1]])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        TileSet { combos, tiles }
    }
}
