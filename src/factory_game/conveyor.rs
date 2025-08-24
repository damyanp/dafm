use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, BaseLayerEntityDespawned,
        helpers::{ConveyorDirection, ConveyorDirections},
    },
};

pub fn conveyor_plugin(app: &mut App) {
    app.register_type::<Conveyor>()
        .init_resource::<TilesToCheck>()
        .add_systems(PreUpdate, update_tiles_to_check);
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

#[allow(dead_code)]
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

    pub fn set_inputs(&mut self, inputs: ConveyorDirections) {
        self.inputs = inputs;
    }

    pub fn set_outputs(&mut self, outputs: ConveyorDirections) {
        self.outputs = outputs;
    }
}

#[derive(Component, Debug, Reflect, Default)]
pub struct SimpleConveyor;

#[derive(Resource, Default)]
pub struct TilesToCheck(pub HashSet<TilePos>);

pub fn update_tiles_to_check(
    mut commands: Commands,
    new: Query<&TilePos, Added<Conveyor>>,
    mut removed: EventReader<BaseLayerEntityDespawned>,
    base: Single<&TilemapSize, With<BaseLayer>>,
) {
    let map_size = base.into_inner();

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

    commands.insert_resource(TilesToCheck(to_check));
}
