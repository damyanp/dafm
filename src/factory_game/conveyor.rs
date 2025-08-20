use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, BaseLayerEntityDespawned, ConveyorSystems,
        helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query},
    },
};

mod transfers;
pub use transfers::*;

mod payloads;
use payloads::*;
pub use payloads::{
    AcceptsPayloadConveyor, PayloadDestination, PayloadOf, PayloadSource, PayloadTransport,
    Payloads, take_payload,
};

pub struct PayloadPlugin;
impl Plugin for PayloadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AcceptsPayloadConveyor>()
            .register_type::<Conveyor>()
            .register_type::<PayloadOf>()
            .register_type::<Payloads>()
            .register_type::<PayloadTransport>()
            .register_type::<PayloadDestination>()
            .register_type::<PayloadSource>()
            .register_type::<DistributorConveyor>()
            .add_event::<RequestPayloadTransferEvent>()
            .add_systems(
                Update,
                (
                    (
                        update_conveyor_inputs,
                        transfer_payloads_standard,
                        update_payload_mus,
                        (
                            update_simple_conveyor_destinations,
                            update_distributor_conveyor_destinations,
                        ),
                    )
                        .chain()
                        .in_set(ConveyorSystems::TransportLogic),
                    update_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
                ),
            );
    }
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
}

#[derive(Component, Debug, Reflect, Default)]
pub struct DistributorConveyor {
    pub next_output: ConveyorDirection,
}

#[derive(Component, Debug, Reflect)]
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
