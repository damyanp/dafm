use bevy::prelude::*;

use crate::conveyor::helpers::ConveyorDirection;

pub struct ConveyorPlugin;
impl Plugin for ConveyorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Conveyor>();
    }
}

#[derive(Component, Clone, Debug, Reflect, Default)]
pub struct Conveyor(pub ConveyorDirection);
