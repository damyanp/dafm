use std::time::Duration;

use bevy::{prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy};
use bevy_ecs_tilemap::{
    map::TilemapSize,
    tiles::{TilePos, TileStorage},
};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, MapConfig, bridge::PlaceBridgeEvent, conveyor_belts::PlaceConveyorBeltEvent,
        generator::PlaceGeneratorEvent, operators::Operand,
    },
};

use super::helpers::ConveyorDirection;

fn setup() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        crate::factory_game::factory_game_logic_plugin,
    ));
    app.init_state::<GameState>()
        .insert_state(GameState::FactoryGame)
        .insert_resource(Time::<Virtual>::from_max_delta(Duration::from_secs(10)))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));

    let map_config = MapConfig::default();

    app.world_mut().spawn((
        BaseLayer,
        TileStorage::empty(map_config.size),
        TilemapSize::from(map_config.size),
    ));

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

#[test]
fn generator_transfers_payload_to_conveyor() {
    let mut app = setup();

    let world = app.world_mut();
    world.trigger(PlaceGeneratorEvent(TilePos { x: 0, y: 0 }));
    world.trigger(PlaceConveyorBeltEvent(
        TilePos { x: 1, y: 0 },
        ConveyorDirection::East,
    ));

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));

    app.update(); // payload spawned
    app.update(); // moving to edge of generator
    app.update(); // ready to be transferred
    app.update(); // transferred to conveyor belt

    // let mut payloads = app
    //     .world_mut()
    //     .query_filtered::<&Payloads, With<ConveyorBelt>>();

    // assert_eq!(payloads.iter(app.world()).len(), 1);
    todo!();
}

#[test]
fn generator_doesnt_transfer_payload_to_conveyor_pointing_at_it() {
    let mut app = setup();

    let world = app.world_mut();
    world.trigger(PlaceGeneratorEvent(TilePos { x: 0, y: 0 }));
    world.trigger(PlaceConveyorBeltEvent(
        TilePos { x: 1, y: 0 },
        ConveyorDirection::West,
    ));

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));

    app.update(); // payload spawned
    app.update(); // moving to edge of generator
    app.update(); // ready to be transferred
    app.update(); // transferred to conveyor belt

    // let mut payloads = app
    //     .world_mut()
    //     .query_filtered::<&Payloads, With<ConveyorBelt>>();

    // assert_eq!(payloads.iter(app.world()).len(), 0);
    todo!();
}

#[test]
fn generator_transfers_payload_to_bridge() {
    let mut app = setup();

    let world = app.world_mut();
    world.trigger(PlaceGeneratorEvent(TilePos { x: 0, y: 0 }));
    world.trigger(PlaceBridgeEvent(TilePos { x: 1, y: 0 }));

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));

    app.update(); // payload spawned
    app.update(); // moving to edge of generator
    app.update(); // ready to be transferred
    app.update(); // transferred to bridge

    // let mut payloads = app.world_mut().query_filtered::<&Payloads, With<Bridge>>();

    // assert_eq!(payloads.iter(app.world()).len(), 1);
    todo!();
}
