use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        distributor::DistributorConveyor,
        helpers::ConveyorDirections,
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        operators::{Operand, operand_bundle},
    },
    sprite_sheet::GameSprite,
};

pub fn generator_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceGeneratorEvent>()
        .register_type::<Generator>()
        .add_systems(
            Update,
            (
                update_generator_tiles.in_set(ConveyorSystems::TileUpdater),
                generate_payloads.in_set(ConveyorSystems::TransferPayloads),
            ),
        );
}

pub struct GeneratorTool;

impl Tool for GeneratorTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Generator, TileFlip::default())
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceGeneratorEvent(*tile_pos));
    }
}

#[derive(Event, Debug)]
pub struct PlaceGeneratorEvent(pub TilePos);

impl PlaceTileEvent for PlaceGeneratorEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((Generator::default(), Name::new("Generator")));
    }
}

#[derive(Component, Debug, Reflect)]
#[require(Conveyor::new(ConveyorDirections::all()), DistributorConveyor::new(5))]
struct Generator {
    next_generate_time: f32,
    time_between_generations: f32,
}

impl Default for Generator {
    fn default() -> Self {
        Generator {
            next_generate_time: 0.0,
            time_between_generations: 1.0,
        }
    }
}

fn update_generator_tiles(
    mut commands: Commands,
    new_generators: Query<Entity, (With<Generator>, Without<TileTextureIndex>)>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TileStorage>)>,
) {
    for new_generator in new_generators {
        commands.entity(new_generator).insert_if_new((TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: GameSprite::Generator.tile_texture_index(),
            ..default()
        },));
    }
}

fn generate_payloads(
    mut commands: Commands,
    time: Res<Time>,
    generators: Query<(
        &TilePos,
        &Conveyor,
        &mut Generator,
        &mut DistributorConveyor,
    )>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    conveyors: Query<&Conveyor>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for (tile_pos, conveyor, mut generator, mut distributor) in generators {
        if time.elapsed_secs() > generator.next_generate_time
            && distributor.try_take(
                conveyor,
                tile_storage,
                tile_pos,
                map_size,
                &conveyors,
                || commands.spawn(operand_bundle(Operand(1))).id(),
            )
        {
            generator.next_generate_time = time.elapsed_secs() + generator.time_between_generations;
        }
    }
}
