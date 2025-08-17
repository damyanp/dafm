use std::time::Duration;

use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_ecs_tilemap::tiles::{TilePos, TileStorage};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, FactoryGameLogicPlugin, MapConfig, generator::PlaceGeneratorEvent,
        operators::Operand,
    },
};

fn setup() -> App {
    let mut app = App::new();

    app.add_plugins((MinimalPlugins, StatesPlugin, FactoryGameLogicPlugin));
    app.init_state::<GameState>()
        .insert_state(GameState::FactoryGame);

    let map_config = MapConfig::default();

    app.world_mut()
        .spawn((BaseLayer, TileStorage::empty(map_config.size)));

    app
}

#[test]
fn generator_generates_payload() {
    let mut app = setup();

    let world = app.world_mut();

    world.trigger(PlaceGeneratorEvent(TilePos { x: 0, y: 0 }));

    world
        .resource_mut::<Time<Virtual>>()
        .advance_by(Duration::from_secs(5));

    app.update();

    let world = app.world_mut();
    let mut q = world.query::<&Operand>();

    assert_eq!(q.iter(world).len(), 1);
    assert_eq!(q.single(world).unwrap().0, 1);
}
