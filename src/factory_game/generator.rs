use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{Conveyor, DistributorConveyor, PayloadOf, PayloadTransport, Payloads},
        helpers::ConveyorDirections,
        interaction::Tool,
    },
    sprite_sheet::GameSprite,
};

pub struct GeneratorPlugin;
impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_generator_tiles.in_set(ConveyorSystems::TileUpdater),
                generate_payloads.in_set(ConveyorSystems::TransportLogic),
            ),
        );
    }
}

pub struct GeneratorTool;

impl Tool for GeneratorTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Generator, TileFlip::default())
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((GeneratorBundle::new(), Name::new("Generator")));
    }
}

#[derive(Component, Default, Debug)]
struct Generator {
    next_generate_time: f32,
}

#[derive(Bundle)]
pub struct GeneratorBundle {
    generator: Generator,
    conveyor: Conveyor,
    distributor: DistributorConveyor,
}

impl GeneratorBundle {
    pub fn new() -> Self {
        GeneratorBundle {
            generator: Generator::default(),
            conveyor: Conveyor {
                outputs: ConveyorDirections::all(),
                accepts_input: false,
            },
            distributor: DistributorConveyor::default(),
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
    generators: Query<(Entity, &mut Generator, Option<&Payloads>)>,
) {
    for (entity, mut generator, payloads) in generators {
        if time.elapsed_secs() > generator.next_generate_time && payloads.is_none() {
            commands.spawn((
                StateScoped(GameState::FactoryGame),
                Name::new("Payload"),
                PayloadOf(entity),
                Text2d::new("X"),
                TextColor(Color::linear_rgb(1.0, 0.4, 0.4)),
                PayloadTransport { mu: 0.5 },
            ));

            generator.next_generate_time = time.elapsed_secs() + 5.0;
        }
    }
}
