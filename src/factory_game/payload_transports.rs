use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    factory_game::{
        helpers::ConveyorDirection,
        payloads::Payload,
    },
    helpers::TilemapQuery,
};

/// Trait for components that manage multiple payload transport lines and can perform common operations
pub trait PayloadTransports {
    /// Update all transport lines with time delta
    fn update_transports(&mut self, t: f32);
    
    /// Get a payload ready to transfer out along with its direction
    fn get_payload_to_transfer(&self) -> Option<(ConveyorDirection, Entity)>;
    
    /// Update visual transforms for all payloads in transport lines
    fn update_all_payload_transforms(
        &self,
        tile_pos: &TilePos,
        payloads: &mut Query<&mut Transform, With<Payload>>,
        base: &crate::helpers::TilemapQueryItem,
    );
    
    /// Remove a payload from all transport lines
    fn remove_payload_from_transports(&mut self, payload: Entity);
    
    /// Iterate over all payloads in transport lines
    fn iter_transport_payloads(&self) -> Box<dyn Iterator<Item = Entity> + '_>;
}

/// Generic system for updating payload transforms for any component that implements PayloadTransports
pub fn update_payload_handler_transforms<T: Component + PayloadTransports>(
    handlers: Query<(&TilePos, &T)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<crate::factory_game::BaseLayer>>,
) {
    for (tile_pos, handler) in handlers.iter() {
        handler.update_all_payload_transforms(tile_pos, &mut payloads, &base);
    }
}