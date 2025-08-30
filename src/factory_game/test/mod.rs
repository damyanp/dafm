use std::time::Duration;

use bevy::{prelude::*, state::app::StatesPlugin, time::TimeUpdateStrategy};
use bevy_ecs_tilemap::tiles::{TilePos, TileStorage};

use crate::{
    GameState,
    factory_game::{
        BaseLayer, MapConfig,
        bridge::{BridgeConveyor, PlaceBridgeEvent},
        conveyor_belts::{ConveyorBelt, PlaceConveyorBeltEvent},
        generator::PlaceGeneratorEvent,
        operators::Operand,
        payload_handler::PayloadHandler,
        payloads::PayloadTransportLine,
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
        map_config.size,
    ));

    app
}

#[test]
fn generator_generates_payload() {
    let mut app = setup();

    let world = app.world_mut();

    world.trigger(PlaceGeneratorEvent(TilePos { x: 0, y: 0 }));
    world.trigger(PlaceConveyorBeltEvent(
        TilePos { x: 1, y: 0 },
        ConveyorDirection::East,
    ));

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

    let mut ptl = app
        .world_mut()
        .query_filtered::<&PayloadTransportLine, With<ConveyorBelt>>();

    assert_eq!(ptl.single(app.world()).unwrap().count(), 1);
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

    let mut ptl = app
        .world_mut()
        .query_filtered::<&PayloadTransportLine, With<ConveyorBelt>>();

    assert_eq!(ptl.single(app.world()).unwrap().count(), 0);
}

#[test]
fn generator_transfers_payload_to_bridge() {
    let mut app = setup();

    app.world_mut()
        .trigger(PlaceBridgeEvent(TilePos { x: 2, y: 1 }));
    app.world_mut()
        .trigger(PlaceGeneratorEvent(TilePos { x: 1, y: 1 }));

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));

    app.update(); // payload spawned
    app.update(); // moving to edge of generator
    app.update(); // ready to be transferred
    app.update(); // transferred to bridge

    let mut bridge = app.world_mut().query::<&BridgeConveyor>();

    let bridge = bridge.single(app.world()).unwrap();
    println!("{bridge:?}");

    assert_eq!(bridge.iter_payloads().count(), 1);
}
