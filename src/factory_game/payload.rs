use bevy::prelude::*;

use crate::factory_game::helpers::ConveyorDirection;

pub struct PayloadPlugin;
impl Plugin for PayloadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PayloadOf>()
            .register_type::<Payloads>()
            .register_type::<PayloadTransport>();
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
    pub source: Option<ConveyorDirection>,
    pub destination: ConveyorDirection,
}

impl PayloadTransport {
    pub fn new(direction: ConveyorDirection) -> Self {
        PayloadTransport {
            source: None,
            destination: direction,
            mu: 0.0,
        }
    }
}
