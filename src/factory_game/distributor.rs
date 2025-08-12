use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};

use crate::factory_game::{
    BaseLayer, BaseLayerEntityDespawned, ConveyorSystems,
    conveyor::Conveyor,
    helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query},
};

pub struct DistributorPlugin;
impl Plugin for DistributorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                find_distributors_to_update.pipe(update_distributors),
                update_distributor_tiles,
            )
                .in_set(ConveyorSystems::TileUpdater),
        );
    }
}

#[derive(Component, PartialEq, Eq, Hash)]
#[require(Conveyor=new_distributor_conveyor())]
pub struct Distributor;

fn new_distributor_conveyor() -> Conveyor {
    Conveyor {
        outputs: ConveyorDirections::default(),
        accepts_input: true,
        next_output: ConveyorDirection::North,
        is_full: true,
    }
}

fn find_distributors_to_update(
    added_distributors: Query<Entity, Added<Distributor>>,
    new_conveyors: Query<&TilePos, Added<Conveyor>>,
    mut removed_entities: EventReader<BaseLayerEntityDespawned>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) -> HashSet<Entity> {
    let (tile_storage, map_size) = base.into_inner();

    let mut to_update = HashSet::new();

    for new_distributor in added_distributors {
        to_update.insert(new_distributor);
    }

    let new_conveyors = new_conveyors.into_iter();
    let removed_conveyors = removed_entities.read().map(|p| &p.0);
    let new_and_removed_conveyors = new_conveyors.chain(removed_conveyors);
    for pos in new_and_removed_conveyors {
        for neighbor_entity in Neighbors::get_square_neighboring_positions(pos, map_size, false)
            .entities(tile_storage)
            .iter()
        {
            to_update.insert(*neighbor_entity);
        }
    }

    to_update
}

fn update_distributors(
    to_update: In<HashSet<Entity>>,
    mut conveyors: Query<(&TilePos, &mut Conveyor, Option<&Distributor>)>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for distributor_entity in to_update.iter() {
        let outputs =
            get_distributor_outputs(*distributor_entity, &conveyors, tile_storage, map_size);

        if let Some(outputs) = outputs {
            if let Ok((_, mut conveyor, _)) = conveyors.get_mut(*distributor_entity) {
                conveyor.accepts_input = !outputs.is_none();
                conveyor.outputs = outputs;
            }
        }
    }
}

fn get_distributor_outputs(
    distributor_entity: Entity,
    conveyors: &Query<(&TilePos, &mut Conveyor, Option<&Distributor>)>,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
) -> Option<ConveyorDirections> {
    if let Ok((tile_pos, _, distributor)) = conveyors.get(distributor_entity)
        && distributor.is_some()
    {
        // the distributor's outputs are all the neighbors that accept inputs
        let neighbors = get_neighbors_from_query(tile_storage, tile_pos, map_size, conveyors);

        let mut directions = ConveyorDirections::default();

        for (direction, (_, conveyor, _)) in neighbors.iter_with_direction() {
            let direction: ConveyorDirection = direction.into();
            if conveyor.accepts_input && !conveyor.outputs.is_set(direction.opposite()) {
                directions.add(direction);
            }
        }
        Some(directions)
    } else {
        None
    }
}

fn update_distributor_tiles(
    mut commands: Commands,
    new_distributors: Query<Entity, Added<Distributor>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TilemapSize>)>,
) {
    for e in new_distributors {
        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: TileTextureIndex(32),
            ..default()
        });
    }
}
