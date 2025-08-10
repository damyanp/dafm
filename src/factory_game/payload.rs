use bevy::prelude::*;

use crate::factory_game::helpers::ConveyorDirection;

pub struct PayloadPlugin;
impl Plugin for PayloadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PayloadOf>()
            .register_type::<Payloads>()
            .register_type::<PayloadTransport>()
            .add_event::<OfferPayloadEvent>()
            .add_event::<TookPayloadEvent>();
    }
}

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Payloads)]
pub struct PayloadOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = PayloadOf)]
pub struct Payloads(Vec<Entity>);

#[derive(Component, Reflect)]
pub struct PayloadTransport {
    pub mu: f32,
    pub source: ConveyorDirection,
    pub destination: ConveyorDirection,
}

#[derive(Event)]
pub struct OfferPayloadEvent {
    pub source_direction: ConveyorDirection,
    pub payload: Entity,
    pub target: Entity,
}

#[derive(Event)]
pub struct TookPayloadEvent {
    pub payload: Entity,
}
