use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::{Neighbors, SquareDirection};
use bevy_ecs_tilemap::prelude::*;

use crate::conveyor::{
    Conveyor, ConveyorSystems, generator::Generator, helpers::ConveyorDirection, visuals::BaseLayer,
};

pub struct PayloadPlugin;
impl Plugin for PayloadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PayloadOf>()
            .register_type::<Payloads>()
            .register_type::<PayloadTransport>()
            .add_systems(
                Update,
                transport_payloads.in_set(ConveyorSystems::TransportLogic),
            );
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
    pub direction: ConveyorDirection,
}

pub fn transport_payloads(
    mut commands: Commands,
    time: Res<Time>,
    payloads: Query<(Entity, &PayloadOf, &mut PayloadTransport)>,
    carriers: Query<(&TilePos, AnyOf<(&Conveyor, &Generator)>)>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let mu_speed = time.delta_secs();

    let (tile_storage, map_size) = base.into_inner();

    for (payload_entity, mut payload_of, mut transport) in payloads {
        let (carrier_pos, _) = carriers
            .get(payload_of.0)
            .expect("Payload must be attached to a conveyor or generator");

        let destination_pos = carrier_pos.square_offset(&transport.direction.into(), map_size);
        let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));

        if let Some(destination_entity) = destination_entity {
            transport.mu += mu_speed;
            if transport.mu > 1.0 {
                transport.mu = 0.0;
                commands
                    .entity(payload_entity)
                    .insert(PayloadOf(destination_entity));
            }
        } else {
            // destination has gone
            transport.mu -= mu_speed;
            if transport.mu < 0.0 {
                commands.entity(payload_entity).remove::<PayloadTransport>();
            }
        }
    }
}
