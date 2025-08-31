use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        helpers::ConveyorDirection,
        payloads::Payload,
    },
    helpers::TilemapQuery,
};

/// Trait for components that manage payload outputs and can perform common payload operations
pub trait PayloadOutputs {
    /// Update payloads with time delta
    fn update_payloads(&mut self, t: f32);
    
    /// Get a payload ready to transfer along with its direction
    fn get_payload_to_transfer(&self) -> Option<(ConveyorDirection, Entity)>;
    
    /// Update visual transforms for all output payloads
    fn update_all_payload_transforms(
        &self,
        tile_pos: &TilePos,
        payloads: &mut Query<&mut Transform, With<Payload>>,
        base: &crate::helpers::TilemapQueryItem,
    );
    
    /// Remove a payload from all outputs
    fn remove_payload_from_outputs(&mut self, payload: Entity);
    
    /// Iterate over all payloads in outputs
    fn iter_output_payloads(&self) -> Box<dyn Iterator<Item = Entity> + '_>;
}

/// Generic system for updating payload transforms for any component that implements PayloadOutputs
pub fn update_payload_handler_transforms<T: Component + PayloadOutputs>(
    handlers: Query<(&TilePos, &T)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<crate::factory_game::BaseLayer>>,
) {
    for (tile_pos, handler) in handlers.iter() {
        handler.update_all_payload_transforms(tile_pos, &mut payloads, &base);
    }
}