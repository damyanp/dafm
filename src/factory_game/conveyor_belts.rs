use bevy::prelude::*;
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::{Neighbors, SquareDirection},
    prelude::*,
};

use crate::{
    factory_game::{
        BaseLayer, BaseLayerEntityDespawned, ConveyorSystems,
        conveyor::{Conveyor, SimpleConveyor, find_tiles_to_check},
        helpers::{
            ConveyorDirection, ConveyorDirections, get_neighbors_from_query, make_east_relative,
            opposite,
        },
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payloads::PayloadTransportLine,
    },
    sprite_sheet::GameSprite,
};

pub fn conveyor_belts_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceConveyorBeltEvent>()
        .add_systems(
            Update,
            (update_conveyor_belt_tiles, update_conveyor_belt_conveyors)
                .in_set(ConveyorSystems::TileUpdater),
        );
}

#[derive(Default)]
pub struct ConveyorBeltTool(ConveyorDirection);

impl Tool for ConveyorBeltTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Arrow, self.0.tile_flip())
    }

    fn next_variant(&mut self) {
        use ConveyorDirection::*;
        self.0 = match self.0 {
            North => East,
            East => South,
            South => West,
            West => North,
        }
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceConveyorBeltEvent(*tile_pos, self.0));
    }
}

#[derive(Event, Debug)]
pub struct PlaceConveyorBeltEvent(pub TilePos, pub ConveyorDirection);

impl PlaceTileEvent for PlaceConveyorBeltEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((conveyor_belt_bundle(self.1), Name::new("Conveyor Belt")));
    }
}

#[derive(Component)]
#[require(SimpleConveyor)]
pub struct ConveyorBelt;

pub fn conveyor_belt_bundle(output: ConveyorDirection) -> impl Bundle {
    (
        ConveyorBelt,
        Conveyor::from(output),
        PayloadTransportLine::new(output, 5),
    )
}

#[expect(clippy::type_complexity)]
fn update_conveyor_belt_conveyors(
    mut conveyors: ParamSet<(Query<&TilePos, Added<Conveyor>>, Query<&mut Conveyor>)>,
    removed_entities: EventReader<BaseLayerEntityDespawned>,
    conveyor_belts: Query<(), With<ConveyorBelt>>,
    base: Single<(Entity, &TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tilemap_entity, tile_storage, map_size) = base.into_inner();

    let to_check = find_tiles_to_check(conveyors.p0(), removed_entities, map_size);
    for tile_pos in to_check {
        if let Some(entity) = tile_storage.get(&tile_pos)
            && conveyor_belts.get(entity).is_ok()
        {
            let directions = find_incoming_directions(
                &tile_pos,
                tile_storage,
                map_size,
                &conveyors.p1().as_readonly(),
            );
            if let Ok(mut conveyor) = conveyors.p1().get_mut(entity) {
                conveyor.set_inputs(directions);
            }
        }
    }
}

fn find_incoming_directions(
    tile_pos: &TilePos,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    conveyors: &Query<&Conveyor>,
) -> ConveyorDirections {
    let neighbors = find_conveyors_outputting_to(tile_pos, tile_storage, map_size, conveyors);
    let directions = neighbors
        .iter_with_direction()
        .map(|(d, _)| ConveyorDirection::from(d));

    directions.into()
}

fn find_conveyors_outputting_to<'a>(
    tile_pos: &TilePos,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    conveyors: &'a Query<&Conveyor>,
) -> Neighbors<&'a Conveyor> {
    // Find the neighbors that have conveyors on them
    let neighbor_conveyors = get_neighbors_from_query(tile_storage, tile_pos, map_size, conveyors);

    // And just the conveyors pointing towards this one

    Neighbors::from_directional_closure(|dir| {
        neighbor_conveyors.get(dir).and_then(|c| {
            if c.outputs().is_set(opposite(dir).into()) {
                Some(*c)
            } else {
                None
            }
        })
    })
}

#[expect(clippy::type_complexity)]
fn update_conveyor_belt_tiles(
    mut commands: Commands,
    new_conveyor_belts: Query<&TilePos, Added<Conveyor>>,
    removed_entities: EventReader<BaseLayerEntityDespawned>,
    conveyors: Query<&Conveyor>,
    conveyor_belts: Query<
        (&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>),
        With<ConveyorBelt>,
    >,
    base: Single<(Entity, &TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tilemap_entity, tile_storage, map_size) = base.into_inner();

    let to_check = find_tiles_to_check(new_conveyor_belts, removed_entities, map_size);

    for pos in to_check {
        if let Some(entity) = tile_storage.get(&pos)
            && let Ok(conveyor_belt) = conveyor_belts.get(entity)
        {
            commands.entity(entity).insert_if_new(TileBundle {
                tilemap_id: TilemapId(tilemap_entity),
                ..default()
            });

            update_conveyor_belt_tile(
                commands.reborrow(),
                entity,
                conveyor_belt,
                &pos,
                tile_storage,
                map_size,
                &conveyors,
            );
        }
    }
}

fn update_conveyor_belt_tile(
    mut commands: Commands,
    entity: Entity,
    conveyor_belt: (&Conveyor, Option<&TileTextureIndex>, Option<&TileFlip>),
    tile_pos: &TilePos,
    tile_storage: &TileStorage,
    map_size: &TilemapSize,
    conveyors: &Query<&Conveyor>,
) {
    let (conveyor, texture_index, flip) = conveyor_belt;

    let out_dir: SquareDirection = conveyor.output().into();

    let neighbor_conveyors =
        find_conveyors_outputting_to(tile_pos, tile_storage, map_size, conveyors);

    // Rotate all of this so that east is always the "out" direction
    let neighbor_conveyors = make_east_relative(neighbor_conveyors, out_dir);

    let (new_sprite, y_flip) = match neighbor_conveyors {
        Neighbors {
            north: None,
            east: None,
            south: None,
            west: Some(_),
            ..
        } => (GameSprite::ConveyorInWOutE, false),
        Neighbors {
            north: None,
            east: _,
            south: Some(_),
            west: Some(_),
            ..
        } => (GameSprite::ConveyorInSWOutE, false),
        Neighbors {
            north: Some(_),
            east: _,
            south: None,
            west: Some(_),
            ..
        } => (GameSprite::ConveyorInSWOutE, true),
        Neighbors {
            north: None,
            east: _,
            south: Some(_),
            west: None,
            ..
        } => (GameSprite::ConveyorInSOutE, false),
        Neighbors {
            north: Some(_),
            east: None,
            south: None,
            west: None,
            ..
        } => (GameSprite::ConveyorInSOutE, true),
        Neighbors {
            north: Some(_),
            east: _,
            south: Some(_),
            west: None,
            ..
        } => (GameSprite::ConveyorInNSOutE, false),
        Neighbors {
            north: Some(_),
            east: _,
            south: Some(_),
            west: Some(_),
            ..
        } => (GameSprite::ConveyorInNSWOutE, false),
        Neighbors {
            north: Some(_),
            east: Some(_),
            south: None,
            west: None,
            ..
        } => (GameSprite::ConveyorInSOutE, true),
        Neighbors {
            north: None,
            east: _,
            south: None,
            west: _,
            ..
        } => (GameSprite::ConveyorInWOutE, false),
    };

    // y_flip indicates if we should flip y for the "east is always out"
    // orientation.  Now we need to rotate the tile so that the out
    // direction is correct.  For North/South this means that y_flip
    // actually becomes an x_flip.
    let new_flip = match out_dir {
        SquareDirection::North => TileFlip {
            x: y_flip,
            y: true,
            d: true,
        },
        SquareDirection::South => TileFlip {
            x: !y_flip,
            y: false,
            d: true,
        },
        SquareDirection::East => TileFlip {
            x: false,
            y: y_flip,
            d: false,
        },
        SquareDirection::West => TileFlip {
            x: true,
            y: !y_flip,
            d: false,
        },
        _ => panic!(),
    };

    let new_index = new_sprite.tile_texture_index();
    if Some(&new_index) != texture_index || Some(&new_flip) != flip {
        commands.entity(entity).insert((new_index, new_flip));
    }
}
