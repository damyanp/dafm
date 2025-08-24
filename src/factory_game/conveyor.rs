use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};

use crate::{
    GameState,
    factory_game::{
        BaseLayerEntityDespawned,
        helpers::{ConveyorDirection, ConveyorDirections},
    },
};

pub fn conveyor_plugin(app: &mut App) {
    app.register_type::<Conveyor>();
}

#[derive(Component, Clone, Debug, Reflect, Default)]
#[require(StateScoped::<GameState>(GameState::FactoryGame))]
pub struct Conveyor {
    outputs: ConveyorDirections,
    inputs: ConveyorDirections,
}

impl From<ConveyorDirection> for Conveyor {
    fn from(direction: ConveyorDirection) -> Self {
        Conveyor::new(ConveyorDirections::new(direction))
    }
}

impl Conveyor {
    pub fn new(outputs: ConveyorDirections) -> Self {
        Conveyor {
            outputs,
            inputs: ConveyorDirections::default(),
        }
    }

    pub fn output(&self) -> ConveyorDirection {
        self.outputs.single()
    }

    pub fn outputs(&self) -> ConveyorDirections {
        self.outputs
    }

    pub fn single_or_no_output(&self) -> Option<ConveyorDirection> {
        if self.outputs.is_none() {
            None
        } else {
            Some(self.outputs.single())
        }
    }

    pub fn inputs(&self) -> ConveyorDirections {
        self.inputs
    }
}

#[derive(Component, Debug, Reflect, Default)]
pub struct SimpleConveyor;

pub fn find_tiles_to_check(
    new: Query<&TilePos, Added<Conveyor>>,
    mut removed: EventReader<BaseLayerEntityDespawned>,
    map_size: &TilemapSize,
) -> HashSet<TilePos> {
    let mut to_check = HashSet::new();

    new.iter().for_each(|pos| {
        to_check.insert(*pos);
    });

    removed.read().for_each(|entity| {
        to_check.insert(entity.0);
    });

    let sources: Vec<_> = to_check.iter().cloned().collect();
    for pos in sources {
        for neighbor_pos in
            Neighbors::get_square_neighboring_positions(&pos, map_size, false).iter()
        {
            to_check.insert(*neighbor_pos);
        }
    }

    to_check
}

